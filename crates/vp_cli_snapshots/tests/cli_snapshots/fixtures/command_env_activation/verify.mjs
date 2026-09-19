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
  env.VP_HOME = path.join(root, kind === 'external' ? 'home' : 'home ' + special);
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
function terminal(executable, args) {
  const result = spawnSync(executable, args, { env, stdio: 'inherit', timeout: 30000 });
  assert.equal(result.status, 0, result.error?.message);
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
console.log('First setup:');
let activation;
if (kind === 'external') {
  console.log('$ vp env list node');
  terminal(binary, ['env', 'list', 'node']);
  // The full PTY snapshot records the printed command. Other cases also parse
  // the guidance and execute it, so escaping remains covered independently.
  const escaped = path.join(config, 'env').replace(/[\\$`"]/g, '\\$&');
  activation = `. "${escaped}"`;
} else {
  const first = captured(['env', 'list', 'node']);
  const setup = first.stderr.replace(/\u001b\[[0-9;]*m/g, '');
  assert.match(setup, /Vite\+ setup complete/);
  activation = setup
    .split('\n')
    .find((line) => line.trim().startsWith(shell === 'fish' || shell === 'nu' ? 'source "' : '. "'))
    ?.trim();
  assert.ok(activation, setup);
  console.log(
    setup
      .split('\n')
      .filter((line) => /Activate Vite\+|^  \. |^  source |new terminal/.test(line))
      .join('\n'),
  );
}
// Resolve the shell using the runner PATH before switching to the stale PATH.
const shellBin = process.env.PATH.split(path.delimiter)
  .map((dir) => path.join(dir, shell))
  .find((file) => fs.existsSync(file));
assert.ok(shellBin, shell);
Object.assign(env, {
  ACTIVATION_BIN: bin,
  ACTIVATION_COMMAND: activation,
});
console.log('Same terminal:');
if (shell === 'fish') {
  terminal(shellBin, ['--no-config', 'session.fish']);
} else if (shell === 'nu') {
  fs.writeFileSync(
    'activate.nu',
    fs.readFileSync('session.nu', 'utf8').replaceAll('__ACTIVATION_COMMAND__', activation),
  );
  terminal(shellBin, ['--no-config-file', 'activate.nu']);
} else {
  terminal(shellBin, [
    shell === 'bash' ? '--noprofile' : '-f',
    ...(shell === 'bash' ? ['--norc'] : []),
    'session.sh',
  ]);
}
