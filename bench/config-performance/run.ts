import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import {
  appendFileSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  realpathSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs';
import { createRequire } from 'node:module';
import { cpus, tmpdir } from 'node:os';
import path from 'node:path';
import { performance } from 'node:perf_hooks';
import { fileURLToPath } from 'node:url';
import { parseArgs } from 'node:util';

import { compareReports, type BenchmarkReport } from './results.ts';

const { values } = parseArgs({
  options: {
    samples: { type: 'string', default: '15' },
    warmup: { type: 'string', default: '3' },
    output: { type: 'string', default: 'tmp/config-performance' },
    baseline: { type: 'string' },
  },
});
const samples = Number(values.samples);
const warmup = Number(values.warmup);
if (!Number.isInteger(samples) || samples < 1 || !Number.isInteger(warmup) || warmup < 1) {
  throw new Error('--samples and --warmup must be positive integers');
}
if (process.platform === 'win32') {
  throw new Error('This benchmark currently supports Linux and macOS');
}

const repo = fileURLToPath(new URL('../../', import.meta.url));
const cliPackage = path.join(repo, 'packages/cli');
const cli = path.join(cliPackage, 'bin/vp');
if (!existsSync(path.join(cliPackage, 'dist/bin.js'))) {
  throw new Error('Build the checkout first: pnpm -F vite-plus build');
}
const require = createRequire(path.join(cliPackage, 'package.json'));
const output = path.resolve(values.output);
mkdirSync(output, { recursive: true });

// Keep fixtures outside the checkout: Oxc ancestor discovery can find its config.
const temporary = realpathSync(mkdtempSync(path.join(tmpdir(), 'vp-config-performance-')));
const fixture = path.join(temporary, 'project');
const app = path.join(fixture, 'packages/app');
const bin = path.join(temporary, 'bin');
const log = path.join(temporary, 'config-loads.jsonl');
const files = Array.from({ length: 3 }, (_, i) => `packages/app/src/f${i}.ts`);
const source = (i: number) => `export const f${i}={message:"fixture",value:${i}}\n`;
const configs = {
  none: undefined,
  minimal: "export default { staged: { '*.ts': 'vp check --fix' } };\n",
  defined:
    "import { defineConfig } from 'vite-plus';\nexport default defineConfig({ staged: { '*.ts': 'vp check --fix' } });\n",
  blocks: "export default { staged: { '*.ts': 'vp check --fix' }, lint: {}, fmt: {} };\n",
  noop: "export default { staged: { '*.ts': 'node -e \"process.exit(0)\"' } };\n",
};
const cases = [
  { id: 'root/check/no-config', config: 'none', command: 'check', package: false, maxLoads: 0 },
  { id: 'root/check/minimal', config: 'minimal', command: 'check', package: false, maxLoads: 4 },
  {
    id: 'root/check/defineConfig',
    config: 'defined',
    command: 'check',
    package: false,
    maxLoads: 4,
  },
  { id: 'package/check/minimal', config: 'minimal', command: 'check', package: true, maxLoads: 7 },
  { id: 'package/check/blocks', config: 'blocks', command: 'check', package: true, maxLoads: 7 },
  { id: 'root/fmt/minimal', config: 'minimal', command: 'fmt', package: false, maxLoads: 1 },
  { id: 'root/lint/minimal', config: 'minimal', command: 'lint', package: false, maxLoads: 1 },
  { id: 'root/staged/minimal', config: 'minimal', command: 'staged', package: false, maxLoads: 5 },
  { id: 'root/staged/noop', config: 'noop', command: 'staged', package: false, maxLoads: 1 },
] as const;

const env = { ...process.env };
for (const key of Object.keys(env)) {
  if (
    key.startsWith('VP_') ||
    key.startsWith('GIT_') ||
    key === 'DEBUG' ||
    key === 'NODE_OPTIONS'
  ) {
    delete env[key];
  }
}
delete env.NODE_DISABLE_COMPILE_CACHE;
Object.assign(env, {
  PATH: `${bin}${path.delimiter}${env.PATH ?? ''}`,
  NODE_COMPILE_CACHE: path.join(temporary, 'compile-cache'),
  NO_COLOR: '1',
  FORCE_COLOR: '0',
});

function run(program: string, args: string[], cwd: string) {
  const start = performance.now();
  const child = spawnSync(program, args, { cwd, env, encoding: 'utf8', timeout: 60_000 });
  const elapsed = performance.now() - start;
  if (child.error || child.status !== 0) {
    throw new Error(
      `${program} ${args.join(' ')} failed (${child.status}, ${child.signal}):\n${child.error ?? ''}\n${child.stdout}\n${child.stderr}`,
    );
  }
  return { elapsed, stdout: child.stdout };
}
const git = (...args: string[]) =>
  run('git', ['-c', 'core.hooksPath=/dev/null', ...args], fixture).stdout;

function prepare(item: (typeof cases)[number], instrument: boolean) {
  const config = configs[item.config];
  const configPath = path.join(fixture, 'vite.config.ts');
  rmSync(configPath, { force: true });
  if (config !== undefined) {
    const probe = instrument
      ? "import { appendFileSync } from 'node:fs';\nappendFileSync(process.env.VP_BENCH_CONFIG_LOG, JSON.stringify({ pid: process.pid, argv: process.argv }) + '\\n');\n"
      : '';
    writeFileSync(configPath, probe + config);
  }
  for (const [i, file] of files.entries()) {
    writeFileSync(path.join(fixture, file), source(i));
  }
  if (item.command === 'staged') {
    git('add', '--', ...files);
  }
  if (instrument) {
    writeFileSync(log, '');
    env.VP_BENCH_CONFIG_LOG = log;
  } else {
    delete env.VP_BENCH_CONFIG_LOG;
  }
}

function measure(item: (typeof cases)[number]) {
  const args: string[] = [cli, item.command];
  if (item.command === 'check' || item.command === 'lint') {
    args.push('--fix');
  }
  if (item.command !== 'staged') {
    args.push(...files.map((file) => (item.package ? path.relative('packages/app', file) : file)));
  }
  const { elapsed } = run(process.execPath, args, item.package ? app : fixture);
  // A fast no-op or an empty staged-file set is not a successful benchmark.
  if (item.command !== 'lint' && item.config !== 'noop') {
    for (const file of files) {
      const formatted =
        item.command === 'staged'
          ? git('show', `:${file}`)
          : readFileSync(path.join(fixture, file), 'utf8');
      if (!formatted.includes('= {')) {
        throw new Error(`${item.id} did not format ${file}`);
      }
    }
  }
  return elapsed;
}

const report: BenchmarkReport = {
  schemaVersion: 1,
  workload: createHash('sha256')
    .update(
      JSON.stringify({
        configs,
        cases: cases.map(({ id, config, command, package: fromPackage }) => ({
          id,
          config,
          command,
          fromPackage,
        })),
        source: files.map((_, i) => source(i)),
      }),
    )
    .digest('hex'),
  revision: run('git', ['rev-parse', 'HEAD'], repo).stdout.trim(),
  environment: {
    node: process.version,
    platform: process.platform,
    arch: process.arch,
    cpu: cpus()[0].model,
    cpus: cpus().length,
  },
  versions: Object.fromEntries(
    ['vite-plus', 'vite', 'oxlint', 'oxfmt'].map((name) => [
      name,
      (require(`${name}/package.json`) as { version: string }).version,
    ]),
  ),
  results: cases.map(({ id }) => ({ id, samples: [], configLoads: [] })),
};

try {
  mkdirSync(path.join(app, 'src'), { recursive: true });
  mkdirSync(path.join(fixture, 'node_modules'), { recursive: true });
  mkdirSync(bin);
  symlinkSync(cliPackage, path.join(fixture, 'node_modules/vite-plus'), 'dir');
  symlinkSync(cli, path.join(bin, 'vp'));
  symlinkSync(process.execPath, path.join(bin, 'node'));
  writeFileSync(
    path.join(fixture, 'package.json'),
    JSON.stringify({ private: true, type: 'module', workspaces: ['packages/*'] }),
  );
  writeFileSync(path.join(fixture, 'pnpm-workspace.yaml'), 'packages:\n  - packages/*\n');
  writeFileSync(
    path.join(app, 'package.json'),
    JSON.stringify({ name: 'config-perf-app', private: true, type: 'module' }),
  );
  writeFileSync(path.join(fixture, '.gitignore'), 'node_modules\nvite.config.ts\n');
  for (const [i, file] of files.entries()) {
    writeFileSync(path.join(fixture, file), `export const f${i} = ${i};\n`);
  }
  git('init', '--quiet');
  git('add', '.');
  git(
    '-c',
    'user.name=Benchmark',
    '-c',
    'user.email=benchmark@example.invalid',
    '-c',
    'commit.gpgsign=false',
    'commit',
    '--quiet',
    '-m',
    'fixture',
  );

  // Separate diagnostic probes keep synchronous log writes out of timed configs.
  for (const [index, item] of cases.entries()) {
    prepare(item, true);
    measure(item);
    const entries = readFileSync(log, 'utf8')
      .trim()
      .split('\n')
      .filter(Boolean)
      .map((line) => JSON.parse(line) as { pid: number; argv: string[] });
    const processes = new Map<number, { role: string; count: number }>();
    for (const entry of entries) {
      const executable = path.basename(entry.argv[1]);
      const role = executable === 'oxfmt' || executable === 'oxlint' ? executable : entry.argv[2];
      const current = processes.get(entry.pid) ?? { role, count: 0 };
      current.count++;
      processes.set(entry.pid, current);
    }
    const loads = [...processes.values()];
    report.results[index].configLoads = loads;
    if (
      entries.length > item.maxLoads ||
      loads.some(({ role, count }) => count > (role === 'check' && item.package ? 4 : 1))
    ) {
      throw new Error(`${item.id}: config evaluation budget exceeded: ${JSON.stringify(loads)}`);
    }
  }

  // Rotate cases between rounds to spread thermal and runner-load changes.
  for (let round = 0; round < warmup + samples; round++) {
    for (let offset = 0; offset < cases.length; offset++) {
      const index = (round + offset) % cases.length;
      prepare(cases[index], false);
      const elapsed = measure(cases[index]);
      if (round >= warmup) {
        report.results[index].samples.push(elapsed);
      }
    }
    console.log(
      `Completed ${round < warmup ? 'warmup' : 'sample'} round ${round + 1}/${warmup + samples}`,
    );
  }
  const baseline = values.baseline
    ? (JSON.parse(readFileSync(values.baseline, 'utf8')) as BenchmarkReport)
    : undefined;
  const { regressions, notableChanges, markdown } = compareReports(report, baseline);
  writeFileSync(path.join(output, 'results.json'), `${JSON.stringify(report, null, 2)}\n`);
  writeFileSync(path.join(output, 'summary.md'), markdown);
  const comment =
    notableChanges.length > 0
      ? `${notableChanges.length} case(s) changed by more than ±5% in median time. Negative changes are faster; positive changes are slower.\n\n${markdown}`
      : '';
  writeFileSync(path.join(output, 'comment.md'), comment);
  if (process.env.GITHUB_OUTPUT) {
    appendFileSync(process.env.GITHUB_OUTPUT, 'report-ready=true\n');
  }
  if (process.env.GITHUB_STEP_SUMMARY) {
    appendFileSync(process.env.GITHUB_STEP_SUMMARY, markdown);
  }
  console.log(markdown);
  if (regressions.length > 0) {
    process.exitCode = 1;
  }
} catch (error) {
  writeFileSync(path.join(output, 'failure.txt'), String(error));
  writeFileSync(path.join(output, 'results.json'), `${JSON.stringify(report, null, 2)}\n`);
  throw error;
} finally {
  rmSync(temporary, { recursive: true, force: true });
}
