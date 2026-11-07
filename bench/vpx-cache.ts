/**
 * Compare a preserved baseline package with the vpx cache prototype.
 *
 * node bench/vpx-cache.ts --baseline-package /tmp/baseline/cli \
 *   --candidate-package ./packages/cli --node /absolute/path/to/node \
 *   --runs 50 --json /tmp/vpx-cache-results.json
 *
 * Both packages need built dist/ and binding/ files. The optional --vpx executable
 * adds global-launcher rows; its isolated config selects the requested Node.js
 * through system-first mode. Every launch verifies the actual executable.
 *
 * Cold samples clear only the contender's disposable cache and temporary files.
 * Warm samples use fresh processes after completed priming runs. Baseline,
 * disabled, transform-only, compile-only, and combined modes run in seeded random
 * blocks. The five historical fixtures are shared with vpx-script.ts.
 */

import { spawn, spawnSync } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  realpathSync,
  rmSync,
  statSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs';
import { arch, cpus, platform, release, tmpdir, totalmem } from 'node:os';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { parseArgs } from 'node:util';

import { generateVpxFixtures, vpxFixtures } from './vpx-fixtures.ts';

const { values } = parseArgs({
  options: {
    'baseline-package': { type: 'string' },
    'candidate-package': { type: 'string' },
    node: { type: 'string', multiple: true },
    vpx: { type: 'string' },
    runs: { type: 'string', default: '50' },
    warmup: { type: 'string', default: '3' },
    'cold-runs': { type: 'string', default: '5' },
    'timeout-ms': { type: 'string', default: '30000' },
    seed: { type: 'string', default: '1' },
    scenario: { type: 'string', multiple: true },
    launcher: { type: 'string', multiple: true },
    mode: { type: 'string', multiple: true },
    json: { type: 'string' },
    diagnostics: { type: 'boolean', default: false },
    keep: { type: 'boolean', default: false },
    help: { type: 'boolean', default: false },
  },
});

if (values.help) {
  console.log(`Usage: node bench/vpx-cache.ts --baseline-package <built cli> --json <file>
  --candidate-package <built cli>  Default: this checkout's packages/cli
  --node <executable>              Repeatable; default: current Node.js
  --vpx <executable>               Also test the global launcher in system-first mode
  --runs <n> --warmup <n>          Warm measured runs and priming runs (50, 3)
  --cold-runs <n>                  Empty-cache measured runs (5; 0 to skip)
  --scenario <name>                Repeatable; default: all five historical fixtures
  --launcher <direct|local|global> Repeatable; default: direct, local, and supplied global
  --mode <name>                    baseline, disabled, transform, compile, combined
  --seed <n>                       Deterministic shuffle seed (1)
  --timeout-ms <n>                 Per-launch timeout (30000)
  --diagnostics                   Extra untimed runs with native cacheStats, if available
  --keep                          Retain the temporary fixture/cache tree`);
  process.exit(0);
}

function integer(value: string, name: string, minimum = 1): number {
  const number = Number(value);
  if (!Number.isSafeInteger(number) || number < minimum) {
    throw new Error(`${name} must be an integer >= ${minimum}`);
  }
  return number;
}

const runs = integer(values.runs, '--runs');
const warmup = integer(values.warmup, '--warmup');
const coldRuns = integer(values['cold-runs'], '--cold-runs', 0);
const timeoutMs = integer(values['timeout-ms'], '--timeout-ms');
const seed = integer(values.seed, '--seed', 0);
const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
if (!values['baseline-package'] || !values.json) {
  throw new Error('--baseline-package and --json are required; use --help for usage');
}
const baseline = realpathSync(values['baseline-package']);
const candidate = realpathSync(values['candidate-package'] ?? path.join(repo, 'packages/cli'));
const jsonPath = path.resolve(values.json);
const globalVpx = values.vpx && path.resolve(values.vpx);
const launchers = values.launcher ?? ['direct', 'local', ...(globalVpx ? ['global'] : [])];
const modes = values.mode ?? ['baseline', 'disabled', 'transform', 'compile', 'combined'];
const scenarios = vpxFixtures.filter(
  (fixture) => !values.scenario || values.scenario.includes(fixture.name),
);
for (const name of values.scenario ?? []) {
  if (!vpxFixtures.some((fixture) => fixture.name === name)) {
    throw new Error(`Unknown scenario: ${name}`);
  }
}
for (const name of launchers) {
  if (!['direct', 'local', 'global'].includes(name) || (name === 'global' && !globalVpx)) {
    throw new Error(`Invalid launcher or missing --vpx: ${name}`);
  }
}
for (const name of modes) {
  if (!['baseline', 'disabled', 'transform', 'compile', 'combined'].includes(name)) {
    throw new Error(`Unknown cache mode: ${name}`);
  }
}

// Do not inherit loader/config/cache selectors from the developer's shell. A
// separate temporary directory also isolates the old local launcher's default
// Node compile cache without enabling it in the baseline script process.
const cleanEnv = Object.fromEntries(
  Object.entries(process.env).filter(
    ([key]) =>
      !/^(?:VP.*|OXC_.*|TSX_.*|TS_NODE_.*|NAPI_RS_.*|NODE_OPTIONS|NODE_PATH|NODE_COMPILE_CACHE.*|NODE_DISABLE_COMPILE_CACHE|NODE_V8_COVERAGE)$/i.test(
        key,
      ),
  ),
);

interface Runtime {
  requested: string;
  execPath: string;
  node: string;
  v8: string;
  platform: string;
  arch: string;
  sha256: string;
}

function digest(file: string): string {
  return createHash('sha256').update(readFileSync(file)).digest('hex');
}

function inspectRuntime(requested: string): Runtime {
  const probe = spawnSync(
    requested,
    [
      '-p',
      `JSON.stringify({execPath:process.execPath,node:process.versions.node,v8:process.versions.v8,platform:process.platform,arch:process.arch})`,
    ],
    {
      env: cleanEnv,
      encoding: 'utf8',
      timeout: timeoutMs,
    },
  );
  if (probe.status !== 0 || probe.stderr || probe.error) {
    throw new Error(`Cannot inspect Node.js ${requested}: ${probe.error ?? probe.stderr}`);
  }
  const info = JSON.parse(probe.stdout) as Omit<Runtime, 'requested' | 'sha256'>;
  return {
    requested,
    ...info,
    execPath: realpathSync(info.execPath),
    sha256: digest(info.execPath),
  };
}

function inspectPackage(packageDir: string) {
  const entries = [
    'dist/script-preload.cjs',
    'dist/script-register.js',
    'dist/script-esm-hooks.js',
    'dist/vpx-bin.js',
    'bin/vpx',
    'binding/index.cjs',
  ];
  const bindingDir = path.join(packageDir, 'binding');
  const native = readdirSync(bindingDir).filter((file) => file.endsWith('.node'));
  const files = [...entries, ...native.map((file) => `binding/${file}`)];
  return {
    path: packageDir,
    version: (
      JSON.parse(readFileSync(path.join(packageDir, 'package.json'), 'utf8')) as { version: string }
    ).version,
    files: Object.fromEntries(
      files.map((file) => [
        file,
        {
          sha256: digest(path.join(packageDir, file)),
          bytes: statSync(path.join(packageDir, file)).size,
        },
      ]),
    ),
  };
}

const runtimes = (values.node ?? [process.execPath]).map(inspectRuntime);
const root = mkdtempSync(path.join(tmpdir(), 'vite-plus-bench-vpx-cache-'));
const fixtures = path.join(root, 'fixtures');
const packageLink = path.join(fixtures, 'node_modules/vite-plus');
// Plain JavaScript under node_modules bypasses Oxc's TypeScript-only dependency
// policy. Instrumentation must not add another cacheable transform to hello.
const probeFile = path.join(fixtures, 'node_modules/vpx-benchmark-probe/probe.cjs');
const marker = '__VPX_BENCH_READY__';
mkdirSync(path.dirname(packageLink), { recursive: true });
mkdirSync(path.dirname(probeFile), { recursive: true });
writeFileSync(path.join(path.dirname(probeFile), 'package.json'), '{"type":"commonjs"}\n');
generateVpxFixtures(fixtures, "globalThis[Symbol.for('vpx:benchmark:ready')]();\n");
writeFileSync(
  probeFile,
  `const fs = require('node:fs');
globalThis[Symbol.for('vpx:benchmark:ready')] = () => {
  const result = {
    token: process.env.VPX_BENCH_TOKEN,
    execPath: process.execPath,
    node: process.versions.node,
    v8: process.versions.v8,
    preload: process.execArgv[process.execArgv.indexOf('--require') + 1],
  };
  if (process.env.VPX_BENCH_BINDING) {
    result.cacheStats = require(process.env.VPX_BENCH_BINDING).oxcNode.cacheStats?.();
  }
  process.stdout.write(${JSON.stringify(marker)} + JSON.stringify(result) + '\\n');
  fs.writeFileSync(process.env.VPX_BENCH_OUTPUT, result.token);
};
`,
);

interface Command {
  id: string;
  runtime: Runtime;
  scenario: (typeof vpxFixtures)[number];
  mode: string;
  launcher: string;
  packageDir: string;
  home: string;
  program: string;
  args: string[];
  env: NodeJS.ProcessEnv;
}

type Phase = 'cold' | 'prime' | 'warm' | 'diagnostic';
interface Sample {
  sequence: number;
  timestamp: string;
  command: string;
  phase: Phase;
  iteration: number;
  readyMs: number;
  exitMs: number;
  cacheStats?: unknown;
}

const commands: Command[] = [];
for (const [runtimeIndex, runtime] of runtimes.entries()) {
  for (const scenario of scenarios) {
    for (const launcher of launchers) {
      for (const mode of modes) {
        const packageDir = mode === 'baseline' ? baseline : candidate;
        const id = `${runtimeIndex}/${scenario.name}/${launcher}/${mode}`;
        const home = path.join(root, 'homes', id);
        mkdirSync(path.join(home, 'tmp'), { recursive: true });
        // VP_HOME is intentionally temporary; the global launcher selects the
        // exact runtime on PATH rather than downloading a managed installation.
        writeFileSync(path.join(home, 'config.json'), '{"nodeShimMode":"system_first"}\n');
        const env: NodeJS.ProcessEnv = {
          ...cleanEnv,
          VP_HOME: home,
          VP_SELF_SETUP_NO_MODIFY_PATH: '1',
          PATH: [path.dirname(runtime.execPath), cleanEnv.PATH ?? ''].join(path.delimiter),
          TMPDIR: path.join(home, 'tmp'),
          TMP: path.join(home, 'tmp'),
          TEMP: path.join(home, 'tmp'),
        };
        if (mode === 'disabled') {
          env.VPX_DISABLE_CACHE = '1';
        } else if (mode !== 'baseline') {
          env.VPX_CACHE = mode === 'combined' ? '1' : mode;
        }
        const nodeArgs = [
          '--require',
          probeFile,
          path.join(fixtures, scenario.name, scenario.entry),
        ];
        const args =
          launcher === 'direct'
            ? [
                '--require',
                path.join(packageDir, 'dist/script-preload.cjs'),
                '--import',
                pathToFileURL(path.join(packageDir, 'dist/script-register.js')).href,
                ...nodeArgs,
              ]
            : launcher === 'local'
              ? [path.join(packageDir, 'bin/vpx'), ...nodeArgs]
              : nodeArgs;
        commands.push({
          id,
          runtime,
          scenario,
          mode,
          launcher,
          packageDir,
          home,
          program: launcher === 'global' ? globalVpx! : runtime.execPath,
          args,
          env,
        });
      }
    }
  }
}

const samples: Sample[] = [];
const report = {
  schema: 1,
  startedAt: new Date().toISOString(),
  fixtureRoot: root,
  harness: { execPath: realpathSync(process.execPath), node: process.version },
  globalLauncher: globalVpx
    ? { path: globalVpx, realpath: realpathSync(globalVpx), sha256: digest(globalVpx) }
    : undefined,
  machine: {
    platform: platform(),
    release: release(),
    arch: arch(),
    cpu: cpus()[0]?.model,
    logicalCpus: cpus().length,
    memoryBytes: totalmem(),
  },
  packages: { baseline: inspectPackage(baseline), candidate: inspectPackage(candidate) },
  runtimes,
  settings: {
    runs,
    warmup,
    coldRuns,
    seed,
    timeoutMs,
    globalVpx,
    globalRuntimePolicy: 'system_first',
    diagnostics: values.diagnostics,
  },
  commands: commands.map(({ id, mode, launcher, packageDir, program, args }) => ({
    id,
    mode,
    launcher,
    packageDir,
    program,
    args,
  })),
  samples,
  summaries: [] as ReturnType<typeof summarize>[],
  error: undefined as string | undefined,
  completedAt: undefined as string | undefined,
};

function save(): void {
  mkdirSync(path.dirname(jsonPath), { recursive: true });
  writeFileSync(jsonPath, `${JSON.stringify(report, null, 2)}\n`);
}

let randomState = seed >>> 0;
function shuffled<T>(items: T[]): T[] {
  const result = [...items];
  for (let i = result.length - 1; i > 0; i--) {
    randomState = (Math.imul(randomState, 1664525) + 1013904223) >>> 0;
    const j = Math.floor((randomState / 0x100000000) * (i + 1));
    [result[i], result[j]] = [result[j], result[i]];
  }
  return result;
}

function clearCaches(command: Command): void {
  rmSync(path.join(command.home, 'cache'), { recursive: true, force: true });
  rmSync(path.join(command.home, 'tmp'), { recursive: true, force: true });
  mkdirSync(path.join(command.home, 'tmp'), { recursive: true });
}

let selectedPackage: string | undefined;
async function measure(command: Command, phase: Phase, iteration: number): Promise<Sample> {
  if (selectedPackage !== command.packageDir) {
    rmSync(packageLink, { recursive: true, force: true });
    symlinkSync(command.packageDir, packageLink, process.platform === 'win32' ? 'junction' : 'dir');
    selectedPackage = command.packageDir;
  }
  const token = randomUUID();
  const outputFile = path.join(command.home, 'execution-token');
  rmSync(outputFile, { force: true });
  const env = {
    ...command.env,
    VPX_BENCH_TOKEN: token,
    VPX_BENCH_OUTPUT: outputFile,
    ...(phase === 'diagnostic'
      ? { VPX_BENCH_BINDING: path.join(command.packageDir, 'binding/index.cjs') }
      : {}),
  };
  const timestamp = new Date().toISOString();
  const started = performance.now();
  return new Promise((resolve, reject) => {
    const child = spawn(command.program, command.args, {
      cwd: path.join(fixtures, command.scenario.name),
      env,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    let stdout = '';
    let stderr = '';
    let readyMs: number | undefined;
    const timer = setTimeout(() => {
      child.kill('SIGKILL');
      reject(new Error(`${command.id} timed out after ${timeoutMs} ms`));
    }, timeoutMs);
    child.stdout.setEncoding('utf8');
    child.stderr.setEncoding('utf8');
    child.stdout.on('data', (chunk: string) => {
      stdout += chunk;
      if (
        readyMs === undefined &&
        stdout.includes(marker) &&
        stdout.slice(stdout.indexOf(marker)).includes('\n')
      ) {
        readyMs = performance.now() - started;
      }
    });
    child.stderr.on('data', (chunk: string) => {
      stderr += chunk;
    });
    child.on('error', (error) => {
      clearTimeout(timer);
      reject(error);
    });
    child.on('close', (code, signal) => {
      const exitMs = performance.now() - started;
      clearTimeout(timer);
      try {
        const pieces = stdout.split(marker);
        if (
          code !== 0 ||
          signal ||
          stderr ||
          readyMs === undefined ||
          pieces.length !== 2 ||
          pieces[0].trim() !== command.scenario.output
        ) {
          throw new Error(
            `exit=${code} signal=${signal} ready=${readyMs}\nstdout: ${stdout}\nstderr: ${stderr}`,
          );
        }
        const identity = JSON.parse(pieces[1]) as {
          token: string;
          execPath: string;
          node: string;
          v8: string;
          preload: string;
          cacheStats?: unknown;
        };
        if (
          identity.token !== token ||
          !existsSync(outputFile) ||
          readFileSync(outputFile, 'utf8') !== token
        ) {
          throw new Error('The script did not execute its per-run side effect');
        }
        if (
          realpathSync(identity.execPath) !== command.runtime.execPath ||
          identity.node !== command.runtime.node ||
          identity.v8 !== command.runtime.v8
        ) {
          throw new Error(
            `Runtime mismatch: requested ${JSON.stringify(command.runtime)}, observed ${JSON.stringify(identity)}`,
          );
        }
        if (
          realpathSync(identity.preload) !==
          realpathSync(path.join(command.packageDir, 'dist/script-preload.cjs'))
        ) {
          throw new Error(`Loader mismatch: ${identity.preload} is not from ${command.packageDir}`);
        }
        resolve({
          sequence: samples.length,
          timestamp,
          command: command.id,
          phase,
          iteration,
          readyMs,
          exitMs,
          ...(phase === 'diagnostic' ? { cacheStats: identity.cacheStats } : {}),
        });
      } catch (error) {
        reject(new Error(`${command.id} ${phase} #${iteration}: ${String(error)}`));
      }
    });
  });
}

function percentile(numbers: number[], quantile: number): number {
  const sorted = numbers.toSorted((a, b) => a - b);
  const index = (sorted.length - 1) * quantile;
  const lower = Math.floor(index);
  const upper = Math.ceil(index);
  return sorted[lower] + (sorted[upper] - sorted[lower]) * (index - lower);
}

function summarize(command: Command, phase: 'cold' | 'warm') {
  const selected = samples.filter(
    (sample) => sample.command === command.id && sample.phase === phase,
  );
  return {
    command: command.id,
    phase,
    count: selected.length,
    ready: {
      p50: percentile(
        selected.map((sample) => sample.readyMs),
        0.5,
      ),
      p95: percentile(
        selected.map((sample) => sample.readyMs),
        0.95,
      ),
    },
    exit: {
      p50: percentile(
        selected.map((sample) => sample.exitMs),
        0.5,
      ),
      p95: percentile(
        selected.map((sample) => sample.exitMs),
        0.95,
      ),
    },
  };
}

try {
  save();
  for (const [phase, count] of [
    ['cold', coldRuns],
    ['prime', warmup],
    ['warm', runs],
    ['diagnostic', values.diagnostics ? 1 : 0],
  ] as const) {
    for (let iteration = 0; iteration < count; iteration++) {
      for (const command of shuffled(commands)) {
        if (phase === 'cold' || (phase === 'prime' && iteration === 0)) {
          clearCaches(command);
        }
        samples.push(await measure(command, phase, iteration));
      }
      save();
      console.error(
        `${phase} block ${iteration + 1}/${count}: ${commands.length} validated launches`,
      );
    }
  }
  report.summaries = commands.flatMap((command) =>
    coldRuns
      ? [summarize(command, 'cold'), summarize(command, 'warm')]
      : [summarize(command, 'warm')],
  );
  report.completedAt = new Date().toISOString();
  save();
  console.log(
    '| Runtime / fixture / launcher / mode | State | Ready p50 | Ready p95 | Exit p50 | Exit p95 |',
  );
  console.log('| --- | --- | ---: | ---: | ---: | ---: |');
  for (const summary of report.summaries) {
    console.log(
      `| ${summary.command} | ${summary.phase} | ${summary.ready.p50.toFixed(2)} ms | ${summary.ready.p95.toFixed(2)} ms | ${summary.exit.p50.toFixed(2)} ms | ${summary.exit.p95.toFixed(2)} ms |`,
    );
  }
  console.error(`Results: ${jsonPath}`);
  if (!values.keep) {
    rmSync(root, { recursive: true, force: true });
  } else {
    console.error(`Fixtures and caches retained: ${root}`);
  }
} catch (error) {
  report.error = String(error);
  save();
  console.error(`Failed benchmark retained fixtures and caches at ${root}`);
  throw error;
}
