import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const shared = process.argv[2] === 'shared';
const windows = process.platform === 'win32';
const binaryName = windows ? 'vp.exe' : 'vp';
const source = fs.realpathSync(path.join(process.env.VP_HOME, 'bin', binaryName));
const root = fs.mkdtempSync(path.resolve('native-ownership-'));
const user = path.join(root, 'user');
const home = path.join(user, 'vp-home');
const project = path.join(root, 'project');
const emptyPath = path.join(root, 'empty-path');
const external = path.join(root, 'download', binaryName);

function write(file, contents) {
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, contents);
}

const env = { ...process.env };
for (const key of Object.keys(env)) {
  if (key.startsWith('VP_')) delete env[key];
}
Object.assign(env, {
  HOME: user,
  USERPROFILE: user,
  ZDOTDIR: user,
  XDG_CONFIG_HOME: path.join(user, '.config'),
  XDG_DATA_HOME: path.join(user, '.local/share'),
  XDG_STATE_HOME: path.join(user, '.local/state'),
  XDG_CACHE_HOME: path.join(user, '.cache'),
  APPDATA: path.join(user, 'AppData/Roaming'),
  LOCALAPPDATA: path.join(user, 'AppData/Local'),
  VP_HOME: home,
  VP_SELF_SETUP_NO_MODIFY_PATH: '1',
  VP_NODE_MANAGER: 'yes',
  VP_PM_MANAGER: 'yes',
  VP_NODE_DIST_MIRROR: 'http://127.0.0.1:9',
  NPM_CONFIG_REGISTRY: 'http://127.0.0.1:9',
  npm_config_registry: 'http://127.0.0.1:9',
  PATH: emptyPath,
  CI: 'true',
  NO_COLOR: '1',
});

function run(binary, args) {
  const result = spawnSync(binary, args, { cwd: project, env, encoding: 'utf8', timeout: 30000 });
  assert.equal(result.status, 0, `${result.error ?? ''}\n${result.stdout}\n${result.stderr}`);
}

try {
  for (const directory of [user, project, emptyPath, path.dirname(external)]) {
    fs.mkdirSync(directory, { recursive: true });
  }
  fs.copyFileSync(source, external);
  fs.chmodSync(external, 0o755);
  write(path.join(path.dirname(external), 'vp-install-lock.yaml'), "lockfileVersion: '9.0'\n");
  if (windows) {
    fs.copyFileSync(
      path.join(path.dirname(source), 'vp-shim.exe'),
      path.join(path.dirname(external), 'vp-shim.exe'),
    );
  }
  if (shared) write(path.join(home, 'keep.txt'), 'personal data\n');

  run(external, ['env', '--help']);
  const installed = fs.realpathSync(path.join(home, 'current'));
  assert.equal(fs.existsSync(path.join(home, '.vp-owned')), !shared);
  assert.ok(fs.existsSync(path.join(installed, '.vp-owned')));
  assert.ok(fs.existsSync(path.join(installed, 'bin/.vp-setup-complete')));
  for (const directory of [
    path.join(installed, 'node_modules'),
    path.join(home, 'js_runtime'),
    path.join(home, 'package_manager'),
  ]) {
    assert.equal(fs.existsSync(directory), false, directory);
  }
  console.log('Native setup records version ownership without Node, pnpm, or JavaScript dependencies.');

  run(path.join(installed, 'bin', binaryName), ['implode', '--yes']);
  for (const entry of [installed, path.join(home, 'current'), path.join(home, 'bin', binaryName)]) {
    assert.equal(fs.lstatSync(entry, { throwIfNoEntry: false }), undefined, entry);
  }
  console.log('Uninstall removes the native version, current link, and vp shim.');
  if (shared) {
    assert.equal(fs.readFileSync(path.join(home, 'keep.txt'), 'utf8'), 'personal data\n');
    assert.equal(fs.existsSync(path.join(home, '.vp-owned')), false);
    console.log('The shared root and personal file remain unowned and unchanged.');
  } else {
    assert.equal(fs.existsSync(home), false);
    console.log('The fresh owned root is removed.');
  }
} finally {
  fs.rmSync(root, { recursive: true, force: true, maxRetries: 10, retryDelay: 100 });
}

if (process.env.VP_EMIT_MILESTONES === '1') {
  const name = Buffer.from('native-ownership:verify:complete').toString('base64url');
  process.stdout.write(`\x1b]2;vt-pty-terminal-test:${'0'.repeat(32)}:${name}\x1b\\`);
}
