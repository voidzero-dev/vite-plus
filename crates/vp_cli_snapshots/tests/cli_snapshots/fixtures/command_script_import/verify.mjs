import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

if (process.env.EXPECTED_NODE_VERSION) {
  assert.equal(process.versions.node, process.env.EXPECTED_NODE_VERSION);
}
const require = createRequire(import.meta.url);
assert.throws(() => require.resolve('tsx'), { code: 'MODULE_NOT_FOUND' });
const registerPath = require.resolve('vite-plus/register');
assert.match(registerPath.replaceAll('\\', '/'), /\/dist\/register\.js$/);
assert.equal(import.meta.resolve('vite-plus/register'), pathToFileURL(registerPath).href);

function write(file, contents) {
  mkdirSync(path.dirname(file), { recursive: true });
  writeFileSync(file, contents);
}

// Local packages exercise installed TypeScript without a network dependency.
write('node_modules/fixture-ts-import/package.json', JSON.stringify({
  name: 'fixture-ts-import', type: 'module',
  exports: { '.': './index.ts', './error': './error.ts' },
}));
write('node_modules/fixture-ts-import/index.ts',
  "export enum Kind { Value = 'esm dependency' }\nexport const kind: Kind = Kind.Value;\n");
write('node_modules/fixture-ts-import/error.ts',
  "enum Failure { Value = 'dependency failure' }\nexport function fail() {\n  throw new Error(Failure.Value);\n}\n");
write('node_modules/fixture-ts-require/package.json', JSON.stringify({
  name: 'fixture-ts-require', main: './index.cts',
}));
write('node_modules/fixture-ts-require/index.cts',
  "enum Kind { Value = 'cjs dependency' }\nmodule.exports = Kind.Value;\n");

const configs = {};
for (const name of ['explicit', 'compat', 'upstream', 'parent', 'worker-one', 'worker-two']) {
  configs[name] = `configs with spaces/${name}.json`;
  write(`src/config-${name}.ts`,
    `export enum Value { Current = '${name}' }\nexport const value = Value.Current;\n`);
  write(configs[name], JSON.stringify({
    extends: '../tsconfig.json',
    compilerOptions: {
      baseUrl: '..', paths: { '@value': [`./src/config-${name}.ts`] },
      useDefineForClassFields: name !== 'worker-one',
    },
  }));
}
write('src/name #%.ts', "enum Value { Current = 'encoded filename' }\nconsole.log(Value.Current);\n");

const baseEnv = { ...process.env };
for (const key of ['NODE_OPTIONS', 'VP_SCRIPT_TSCONFIG', 'TSX_TSCONFIG_PATH', 'OXC_TSCONFIG_PATH', 'TS_NODE_PROJECT']) {
  delete baseEnv[key];
}
function run(args, { env = {}, input, cwd, status = 0 } = {}) {
  const child = spawnSync(process.execPath, args, {
    env: { ...baseEnv, ...env }, input, cwd, encoding: 'utf8', timeout: 30000,
  });
  assert.equal(child.error, undefined, `node ${JSON.stringify(args)}: ${child.error}`);
  assert.equal(child.status, status,
    `node ${JSON.stringify(args)}\nstdout: ${child.stdout}\nstderr: ${child.stderr}`);
  return { out: child.stdout.trim(), err: child.stderr.replaceAll('\\', '/') };
}

const esm = 'esm syntax, resolution, JSX, helpers, and dependencies';
const cjs = 'cjs syntax, nested require, helpers, and dependencies';
assert.equal(run(['--import=vite-plus/register', './src/main.ts']).out, esm);
assert.equal(run(['--import', 'vite-plus/register', './src/main.cts']).out, cjs);
assert.equal(run(['--import', pathToFileURL(registerPath).href, './src/name #%.ts']).out, 'encoded filename');
// A file URL is portable; Node also accepts an absolute POSIX path.
if (process.platform !== 'win32') {
  assert.equal(run(['--import', registerPath, './src/name #%.ts']).out, 'encoded filename');
}
assert.equal(run(['./src/main.ts'], { env: { NODE_OPTIONS: '--import=vite-plus/register' } }).out, esm);
assert.equal(run(['./src/main.cts'], { env: { NODE_OPTIONS: '--import vite-plus/register' } }).out, cjs);
console.log('public package export: both flag forms, resolved URL, NODE_OPTIONS, ESM/CJS');

const preloads = ['--import=vite-plus/register', '--import', './src/setup.ts'];
const args = ['one', '--', '--flag', 'space argument'];
const argv = run([...preloads, './src/argv.ts', ...args], {
  status: 7,
  env: { EXPECTED_EXEC_PATH: process.execPath, EXPECTED_PARENT_PID: String(process.pid) },
});
assert.deepEqual(JSON.parse(argv.out), { args, execArgv: preloads });
assert.deepEqual(JSON.parse(run(['--import', './src/setup.ts', './src/argv.ts', ...args], {
  status: 7,
  env: {
    NODE_OPTIONS: '--import=vite-plus/register',
    EXPECTED_EXEC_PATH: process.execPath, EXPECTED_PARENT_PID: String(process.pid),
  },
}).out), { args, execArgv: ['--import', './src/setup.ts'] });
console.log('subsequent TypeScript preload, argv, execArgv, PID, runtime, and exit status');

const esmEval = "const { format } = await import('./src/module.mts'); console.log(format);";
const cjsEval = "console.log(require('./src/common.cts').format);";
assert.equal(run(['--import=vite-plus/register', '--input-type=module', '--eval', esmEval]).out, 'mts');
assert.equal(run(['--import=vite-plus/register', '--eval', cjsEval]).out, 'cts');
assert.equal(run(['--import=vite-plus/register', '--input-type=module', '-'], { input: esmEval }).out, 'mts');
assert.equal(run(['--import=vite-plus/register', '-'], { input: cjsEval }).out, 'cts');
assert.equal(run(['--input-type=module', '--eval',
  "await import('vite-plus/register'); const { format } = await import('./src/module.mts'); console.log(format);",
]).out, 'mts');
for (const options of [
  { args: ['./tests/enum.test.ts'] },
  { args: [], cwd: path.resolve('tests') },
]) {
  const test = run(['--import=vite-plus/register', '--test', '--test-reporter=tap', ...options.args], options);
  assert.match(test.out, /# tests 1\b/);
  assert.match(test.out, /# fail 0\b/);
}
console.log('eval/stdin imports and requires, dynamic registration, node:test and discovery');

const children = 'fork and ESM/CJS workers inherit registration';
assert.equal(run(['--import=vite-plus/register', './src/children.mjs']).out, children);
assert.equal(run(['./src/children.mjs'], { env: { NODE_OPTIONS: '--import=vite-plus/register' } }).out, children);
assert.equal(run(['./src/spawn.mjs'], { env: { NODE_OPTIONS: '--import=vite-plus/register' } }).out,
  'spawn inherits NODE_OPTIONS registration');
assert.equal(run(['--import=vite-plus/register', './src/children.mjs'], {
  env: { TSX_TSCONFIG_PATH: configs.compat, EXPECTED_CONFIG_VALUE: 'compat' },
}).out, children);
console.log('fork, ESM/CJS workers, spawned Node.js, and inherited custom tsconfig');

assert.equal(run(['--import=vite-plus/register', './src/config-workers.mjs'], {
  env: {
    TSX_TSCONFIG_PATH: configs.parent,
    TS_NODE_PROJECT: configs.parent,
    OXC_TSCONFIG_PATH: configs.parent,
  },
}).out, 'concurrent worker configs isolate ESM aliases and CJS transforms from the parent');
assert.equal(run(['./src/worker-host.mjs']).out, 'a plain JavaScript host registers its TypeScript worker');
assert.equal(run(['--import=vite-plus/register', './src/cluster.mjs'], {
  env: { TSX_TSCONFIG_PATH: configs.compat },
}).out, 'cluster inherits registration and custom tsconfig');
assert.equal(run(['./src/internal-host.mjs']).out, 'the internal loader thread skips public registration');
console.log('worker-specific configs, explicit worker preload, cluster, and internal loader thread');

const dedup = 'registration stays single across repeated and mixed preloads';
assert.equal(run(['--import=vite-plus/register', '--import', 'vite-plus/register', './src/dedup.mjs']).out, dedup);
assert.equal(run(['--import=vite-plus/register', './src/dedup.mjs'], {
  env: { NODE_OPTIONS: '--import=vite-plus/register' },
}).out, dedup);
console.log('repeated flags, repeated dynamic imports, and NODE_OPTIONS/argv overlap');

for (const [value, env] of [
  ['default', {}],
  ['compat', { TSX_TSCONFIG_PATH: configs.compat }],
  ['upstream', { OXC_TSCONFIG_PATH: configs.upstream }],
  ['upstream', { TS_NODE_PROJECT: configs.upstream }],
  ['compat', { TS_NODE_PROJECT: configs.compat, OXC_TSCONFIG_PATH: configs.upstream }],
  ['compat', { TSX_TSCONFIG_PATH: configs.compat, OXC_TSCONFIG_PATH: configs.upstream }],
  ['compat', { VP_SCRIPT_TSCONFIG: '', TSX_TSCONFIG_PATH: configs.compat }],
  ['explicit', { VP_SCRIPT_TSCONFIG: configs.explicit, TSX_TSCONFIG_PATH: configs.compat,
    OXC_TSCONFIG_PATH: configs.upstream, TS_NODE_PROJECT: configs.upstream }],
]) {
  const actual = JSON.parse(run(['--import=vite-plus/register', './src/config.ts'], { env }).out);
  assert.deepEqual(actual, { value,
    explicit: env.VP_SCRIPT_TSCONFIG ?? null, compat: env.TSX_TSCONFIG_PATH ?? null,
    upstream: env.OXC_TSCONFIG_PATH ?? null, tsnode: env.TS_NODE_PROJECT ?? null,
  });
}
console.log('config paths with spaces, explicit/compat/upstream precedence, unchanged environment');

const mapped = run(['--import=vite-plus/register', './src/error.ts'], { status: 1 });
assert.match(mapped.err, /Error: mapped failure/);
assert.match(mapped.err, /at new Broken \([^\n]*\/src\/error\.ts:4:\d+\)/);
const dependencyMapped = run(['--import=vite-plus/register', './src/dependency-error.ts'], { status: 1 });
assert.match(dependencyMapped.err, /Error: dependency failure/);
assert.match(dependencyMapped.err, /at fail \([^\n]*\/node_modules\/fixture-ts-import\/error\.ts:3:\d+\)/);
console.log('source maps identify TypeScript lines in application and dependency errors');
