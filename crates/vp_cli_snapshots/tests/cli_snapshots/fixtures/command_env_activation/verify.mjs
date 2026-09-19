import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const [kind, shell] = process.argv.slice(2);
const root = path.resolve('activation');
const binary = path.join(root, 'prefix/bin/vp');
const source = path.join(process.env.VP_HOME, 'current/bin/vp');
const version = '22.18.0';
const seedNode = path.join(process.env.VP_HOME, 'js_runtime/node', version, 'bin/node');
const special = 'space \'quote\' $cash `tick` "double" \\slash';
const env = { ...process.env };
for (const key of Object.keys(env)) {
  if (key.startsWith('VP_') || key.startsWith('XDG_') || key === 'CI') delete env[key];
}
Object.assign(env, {
  HOME: path.join(root, 'user'),
  VP_CLI_TEST: '1',
  VP_NO_UPDATE_CHECK: '1',
  VP_SELF_SETUP_NO_MODIFY_PATH: '1',
  VP_NODE_MANAGER: 'yes',
  VP_PM_MANAGER: 'no',
  VP_SHELL: shell,
  SHELL: '/login-shell-is-not-the-current-shell/fish',
  NPM_CONFIG_REGISTRY: 'http://127.0.0.1:9',
});
fs.mkdirSync(path.dirname(binary), { recursive: true });
fs.mkdirSync(env.HOME, { recursive: true });
fs.copyFileSync(source, binary);
fs.chmodSync(binary, 0o555);
if (kind === 'external' || kind === 'xdg') {
  const pkg = path.join(root, 'prefix/node_modules/vite-plus');
  fs.mkdirSync(path.join(pkg, 'dist'), { recursive: true });
  fs.writeFileSync(path.join(pkg, 'package.json'), '{"name":"vite-plus"}');
  fs.writeFileSync(path.join(pkg, 'dist/bin.js'), '// External bundle');
  fs.writeFileSync(path.join(root, 'prefix/INSTALL_RECEIPT.json'), '{"homebrew_version":"test"}');
} else {
  env.VP_SKIP_DEPS_INSTALL = '1';
  env.VP_VERSION = 'activation-test';
}
if (kind === 'xdg') {
  env.XDG_CONFIG_HOME = path.join(root, 'config ' + special);
  env.XDG_STATE_HOME = path.join(root, 'state');
  env.VP_BIN_DIR = path.join(root, 'bin ' + special);
  env.VP_DATA_DIR = path.join(root, 'data');
  env.VP_CACHE_DIR = path.join(root, 'cache');
} else if (kind === 'legacy') {
  const legacy = path.join(env.HOME, '.vite-plus');
  fs.mkdirSync(legacy, { recursive: true });
  fs.symlinkSync('activation-test', path.join(legacy, 'current'));
} else {
  env.VP_HOME = path.join(root, 'home ' + special);
}
function captured(args, extra = {}) {
  const result = spawnSync(binary, args, {
    env: { ...env, ...extra },
    encoding: 'utf8',
    timeout: 30000,
  });
  assert.equal(result.status, 0, result.error?.message ?? result.stdout + result.stderr);
  return result;
}
const dirs = Object.fromEntries(
  captured([], { VP_DUMP_DIRS: '1' })
    .stdout.trim()
    .split('\n')
    .map((line) => line.split('\t')),
);
// Query the authoritative resolver instead of assuming a platform's directory layout.
const bin = dirs.bin;
const config = dirs.config;
const data = dirs.data;
assert.ok(bin && config && data, JSON.stringify(dirs));
const runtime = path.join(data, 'js_runtime/node', version, 'bin');
fs.mkdirSync(runtime, { recursive: true });
fs.symlinkSync(seedNode, path.join(runtime, 'node'));
fs.writeFileSync('.node-version', version);
const system = path.join(root, 'system');
fs.mkdirSync(system);
fs.writeFileSync(path.join(system, 'node'), '#!/bin/sh\necho system-node\n', { mode: 0o755 });
env.PATH = system;
const first = captured(['env', 'list', 'node']);
const setup = first.stderr.replace(/\u001b\[[0-9;]*m/g, '');
assert.match(setup, /Vite\+ setup complete/);
assert.doesNotMatch(setup, /not first on PATH/); // no duplicate reminder during handoff
const activation = setup
  .split('\n')
  .find((line) => line.trim().startsWith(shell === 'fish' || shell === 'nu' ? 'source "' : '. "'))
  ?.trim();
assert.ok(activation, setup);
console.log('First setup:');
console.log(
  setup
    .split('\n')
    .filter((line) => /Activate Vite\+|^  \. |^  source |new terminal/.test(line))
    .join('\n'),
);
const settingsFile = path.join(config, 'config.json');
const settings = fs.readFileSync(settingsFile, 'utf8');
const saved = JSON.parse(settings);
assert.equal(saved.nodeShimMode ?? 'managed', 'managed');
assert.deepEqual(Object.values(saved.packageManagerShimModes), Array(4).fill('system_first'));
const setupState = path.join(dirs.state, 'self-setup');
const receipts = fs
  .readdirSync(setupState)
  .map((name) => [name, fs.readFileSync(path.join(setupState, name), 'utf8')]);
const prefixFiles = fs.readdirSync(path.dirname(binary));

// Resolve the shell using the runner PATH before switching to the stale PATH.
const shellBin = process.env.PATH.split(path.delimiter)
  .map((dir) => path.join(dir, shell))
  .find((file) => fs.existsSync(file));
assert.ok(shellBin, shell);
function terminal(args, extra = {}, status = 0, executable = binary, stdio = 'inherit') {
  const result = spawnSync(executable, args, {
    env: { ...env, ...extra },
    stdio,
    encoding: 'utf8',
    timeout: 30000,
  });
  assert.equal(result.status, status, result.error?.message);
  return result;
}
Object.assign(env, {
  ACTIVATION_VP: binary,
  ACTIVATION_BIN: bin,
  ACTIVATION_COMMAND: activation,
  ACTIVATION_SYSTEM: system,
});
console.log('Same terminal:');
if (shell === 'fish') {
  terminal(['--no-config', 'session.fish'], {}, 0, shellBin);
} else if (shell === 'nu') {
  fs.writeFileSync(
    'activate.nu',
    fs.readFileSync('session.nu', 'utf8').replaceAll('__ACTIVATION_COMMAND__', activation),
  );
  terminal(['--no-config-file', 'activate.nu'], {}, 0, shellBin);
} else {
  terminal(
    [
      shell === 'bash' ? '--noprofile' : '-f',
      ...(shell === 'bash' ? ['--norc'] : []),
      'session.sh',
    ],
    {},
    0,
    shellBin,
  );
}
console.log('Another unactivated terminal:');
terminal(['env', 'list', 'node']);
assert.equal(fs.readFileSync(settingsFile, 'utf8'), settings);
for (const [name, receipt] of receipts)
  assert.equal(fs.readFileSync(path.join(setupState, name), 'utf8'), receipt);
assert.deepEqual(fs.readdirSync(path.dirname(binary)), prefixFiles);
assert.ok(!fs.existsSync(path.join(root, 'prefix/bin/.vp-setup-complete')));
if (kind === 'external' || kind === 'xdg') {
  assert.ok(!fs.existsSync(path.join(data, 'current')));
  assert.equal(fs.realpathSync(path.join(bin, 'node')), fs.realpathSync(binary));
} else {
  assert.ok(fs.existsSync(path.join(data, 'current/bin/.vp-setup-complete')));
}
console.log('Preferences, setup state, and installation ownership unchanged.');
fs.symlinkSync(bin, path.join(root, 'bin-alias'));
console.log('Equivalent shim directory on PATH:');
terminal(['env', 'list', 'node'], { PATH: path.join(root, 'bin-alias') + path.delimiter + system });

if (kind === 'external') {
  console.log('JSON output:');
  terminal(['env', 'list', 'node', '--json']);
  console.log('Shell-evaluated output:');
  terminal(['env', 'use', version, '--no-install'], { VP_ENV_USE_EVAL_ENABLE: '1' });
  console.log('CI with a TTY:');
  terminal(['env', 'list', 'node'], { CI: '1' });
  console.log('Redirected stdout, stderr, and stdin:');
  for (const stdio of [
    ['inherit', 'pipe', 'inherit'],
    ['inherit', 'inherit', 'pipe'],
    ['pipe', 'inherit', 'inherit'],
  ]) {
    const result = terminal(['env', 'list', 'node'], {}, 0, binary, stdio);
    if (result.stderr !== null) assert.doesNotMatch(result.stderr, /Activate|not first on PATH/);
    if (result.stdout !== null) assert.doesNotMatch(result.stdout, /Activate|not first on PATH/);
  }
  console.log('Direct node shim:');
  terminal(['--version'], {}, 0, path.join(bin, 'node'));
  console.log('Completion protocol:');
  terminal(['--', 'vp', 'env', 'li'], { VP_COMPLETE: 'bash', _CLAP_COMPLETE_INDEX: '2' });
  console.log('\nInstaller capability protocol:');
  terminal([], { VP_SELF_SETUP_SUPPORT_CHECK: '1' });
  console.log('Installer handoff:');
  const handoff = captured([], { VP_SELF_SETUP_SHELL: 'sh' });
  assert.doesNotMatch(handoff.stderr, /not first on PATH/);
  assert.ok(
    handoff.stdout
      .trim()
      .split('\n')
      .every((line) => /^(INSTALL|SHIM|CACHE|CONFIG|STATE)_DIR=/.test(line)),
  );
  console.log('Shell assignments only; no duplicate activation reminder.');
  console.log('Internal background protocol:');
  terminal(['upgrade', '--background-check']);
  console.log('Quiet mode:');
  terminal(['upgrade', '--silent'], {}, 1);
  console.log('Failing eligible command keeps exit status:');
  terminal(['env', 'clean', 'invalid-scope'], {}, 1);
  console.log('System-first mode:');
  terminal(['env', 'off']);
  terminal(['env', 'list', 'node']);
  fs.writeFileSync(settingsFile, settings);
  console.log('Unknown current shell gets labeled commands despite SHELL=fish:');
  terminal(['env', 'list', 'node'], { VP_SHELL: '' });
  console.log('Missing environment file needs repair:');
  const envFile = path.join(config, 'env');
  fs.renameSync(envFile, envFile + '.saved');
  terminal(['env', 'list', 'node']);
  fs.renameSync(envFile + '.saved', envFile);
  console.log('Missing shim needs repair:');
  fs.unlinkSync(path.join(bin, 'node'));
  terminal(['env', 'list', 'node']);
}
