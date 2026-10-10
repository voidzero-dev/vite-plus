/**
 * Startup benchmark for `vpx <script>` against tsx, behind the numbers in
 * rfcs/vpx-script-execution.md (Performance).
 *
 *   pnpm --filter vite-plus build-ts
 *   node bench/vpx-script.ts --node /path/to/node22/bin/node --node /path/to/node24/bin/node
 *
 * Every scenario runs on each `--node` binary (default: the running one). The
 * scenarios are generated in the OS temp directory, outside any tsconfig.json or
 * package.json that would change how either loader treats them. `packages/cli/dist`
 * is the loader under test, and tsx is the copy `packages/core` depends on. A run
 * fails when the commands of a scenario disagree on output, exit with an error, or
 * warn.
 *
 * Options: `--runs <n>` (default 20) and `--warmup <n>` (default 3) per command,
 * and `--json <file>` to keep every measurement.
 */

import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { parseArgs } from 'node:util';

const repo = path.join(path.dirname(fileURLToPath(import.meta.url)), '..');
const fixtures = path.join(tmpdir(), 'vite-plus-bench-vpx-script');
const dist = path.join(repo, 'packages/cli/dist');
const modules = 300;

const { values } = parseArgs({
  options: {
    node: { type: 'string', multiple: true, default: [process.execPath] },
    runs: { type: 'string', default: '20' },
    warmup: { type: 'string', default: '3' },
    json: { type: 'string' },
  },
});
const runs = Number(values.runs);
const warmup = Number(values.warmup);

const tsxPackage = createRequire(path.join(repo, 'packages/core/package.json')).resolve(
  'tsx/package.json',
);
const tsxDir = path.dirname(tsxPackage);
const tsxVersion = (JSON.parse(readFileSync(tsxPackage, 'utf8')) as { version: string }).version;

function write(file: string, content: string): void {
  mkdirSync(path.dirname(path.join(fixtures, file)), { recursive: true });
  writeFileSync(path.join(fixtures, file), content);
}

function moduleSource(i: number): string {
  return `export interface Item${i} {
  id: number;
  name: string;
}

export enum Kind${i} {
  A = 'a${i}',
  B = 'b${i}',
}

export class Store${i} {
  items: Item${i}[] = [];

  constructor(private readonly prefix: string) {}

  add(id: number): Item${i} {
    const item = { id, name: \`\${this.prefix}-\${id}\` };
    this.items.push(item);
    return item;
  }
}

export function run${i}(): number {
  return new Store${i}(Kind${i}.A).add(${i}).id;
}
`;
}

function generateFixtures(): void {
  rmSync(fixtures, { recursive: true, force: true });
  // One erasable file, so Node.js type stripping can run it too.
  write('hello/package.json', '{ "type": "module" }\n');
  write('hello/hello.ts', "const message: string = 'hello';\nconsole.log(message);\n");

  // Many modules with enums and parameter properties (not erasable), as an ES module
  // package and as a package without `"type"`.
  const index = [
    ...Array.from({ length: modules }, (_, i) => `import { run${i} } from './mod${i}.ts';`),
    'let total = 0;',
    ...Array.from({ length: modules }, (_, i) => `total += run${i}();`),
    "console.log('total', total);",
  ].join('\n');
  for (const [name, packageJson] of [
    ['esm-app', '{ "type": "module" }\n'],
    ['cjs-app', '{ "name": "cjs-app" }\n'],
  ]) {
    write(`${name}/package.json`, packageJson);
    write(`${name}/index.ts`, `${index}\n`);
    for (let i = 0; i < modules; i++) {
      write(`${name}/mod${i}.ts`, moduleSource(i));
    }
  }

  // One 1.8 MB file without import/export, as CommonJS and as an ES module.
  let large = '';
  for (let i = 0; i < 12000; i++) {
    large +=
      `interface Shape${i} { id: number; label: string }\n` +
      `function make${i}(id: number): Shape${i} { return { id, label: 'shape-${i}-' + id.toString(16) }; }\n`;
  }
  large +=
    "let total: number = 0;\nfor (let i = 0; i < 100; i++) total += make0(i).id;\nconsole.log('total', total);\n";
  write('large-cjs/package.json', '{ "name": "large-cjs" }\n');
  write('large-cjs/main.ts', large);
  write('large-esm/package.json', '{ "type": "module" }\n');
  write('large-esm/main.ts', large);
}

const scenarios = [
  { name: 'hello', entry: 'hello.ts' },
  { name: 'esm-app', entry: 'index.ts' },
  { name: 'cjs-app', entry: 'index.ts' },
  { name: 'large-cjs', entry: 'main.ts' },
  { name: 'large-esm', entry: 'main.ts' },
];

interface Command {
  label: string;
  args: string[];
  env: NodeJS.ProcessEnv;
}

function commands(scenario: string, entry: string): Command[] {
  const env = Object.fromEntries(
    Object.entries(process.env).filter(([key]) => !/^(VP|TSX|OXC)_/.test(key)),
  );
  const noCache = { ...env, TSX_DISABLE_CACHE: '1' };
  const loader = [
    '--require',
    path.join(dist, 'script-preload.cjs'),
    '--import',
    pathToFileURL(path.join(dist, 'script-register.js')).href,
  ];
  const tsxLoader = pathToFileURL(path.join(tsxDir, 'dist/loader.mjs')).href;
  const tsxCli = path.join(tsxDir, 'dist/cli.mjs');
  const vpxBin = path.join(repo, 'packages/cli/bin/vpx');
  return [
    // What the global `vpx` execs in place, without the Rust shim's own startup.
    { label: '`vpx` loader (1 process)', args: [...loader, entry], env },
    { label: '`node --import tsx` (1 process)', args: ['--import', tsxLoader, entry], env },
    { label: '`node --import tsx`, no cache', args: ['--import', tsxLoader, entry], env: noCache },
    // Both start the script in a child Node.js process.
    { label: '`vpx` local bin (2 processes)', args: [vpxBin, entry], env },
    { label: '`tsx` CLI (2 processes)', args: [tsxCli, entry], env },
    { label: '`tsx` CLI, no cache', args: [tsxCli, entry], env: noCache },
    ...(scenario === 'hello' ? [{ label: '`node` type stripping', args: [entry], env }] : []),
  ];
}

function time(node: string, command: Command, cwd: string) {
  const start = performance.now();
  const result = spawnSync(node, command.args, { cwd, env: command.env });
  return {
    ms: performance.now() - start,
    status: result.status,
    stdout: result.stdout.toString().trim(),
    stderr: result.stderr.toString().trim(),
  };
}

function median(sorted: number[]): number {
  const middle = Math.floor(sorted.length / 2);
  return sorted.length % 2 ? sorted[middle] : (sorted[middle - 1] + sorted[middle]) / 2;
}

generateFixtures();

const nodes = values.node.map((node) => ({
  node,
  version: spawnSync(node, ['-p', 'process.versions.node']).stdout.toString().trim(),
}));
const results: {
  node: string;
  scenario: string;
  command: string;
  median: number;
  samples: number[];
}[] = [];
for (const { node, version } of nodes) {
  for (const { name, entry } of scenarios) {
    const cwd = path.join(fixtures, name);
    let expected: string | undefined;
    for (const command of commands(name, entry)) {
      const first = time(node, command, cwd);
      expected ??= first.stdout;
      if (first.status !== 0 || first.stdout !== expected || first.stderr.includes('Warning')) {
        throw new Error(
          `${version} ${name} ${command.label}: exit ${first.status}, stdout ${JSON.stringify(first.stdout)}\n${first.stderr}`,
        );
      }
      for (let i = 1; i < warmup; i++) {
        time(node, command, cwd);
      }
      const samples = Array.from({ length: runs }, () => time(node, command, cwd).ms).toSorted(
        (a, b) => a - b,
      );
      results.push({
        node: version,
        scenario: name,
        command: command.label,
        median: median(samples),
        samples,
      });
      console.error(
        `${version} ${name.padEnd(9)} ${command.label.padEnd(36)} ${median(samples).toFixed(1)} ms`,
      );
    }
  }
}

if (values.json) {
  writeFileSync(values.json, `${JSON.stringify({ tsx: tsxVersion, results }, null, 2)}\n`);
}

const versions = nodes.map(({ version }) => version);
console.log(`tsx ${tsxVersion}; median of ${runs} runs after ${warmup} warmup runs\n`);
console.log(`| Scenario | Command | ${versions.map((v) => `Node.js ${v}`).join(' | ')} |`);
console.log(`| --- | --- | ${versions.map(() => '--:').join(' | ')} |`);
for (const { name } of scenarios) {
  const labels = [...new Set(results.filter((r) => r.scenario === name).map((r) => r.command))];
  for (const label of labels) {
    const cells = versions.map((version) => {
      const row = results.find(
        (r) => r.node === version && r.scenario === name && r.command === label,
      );
      return row ? `${row.median.toFixed(0)} ms` : '';
    });
    console.log(`| \`${name}\` | ${label} | ${cells.join(' | ')} |`);
  }
}
