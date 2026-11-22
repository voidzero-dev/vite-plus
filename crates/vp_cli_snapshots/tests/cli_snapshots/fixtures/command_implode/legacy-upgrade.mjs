import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import {
  copyFileSync,
  existsSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs';
import { dirname, join } from 'node:path';

// Model 1.4.0's records and its handoff to the new binary without registry downloads.
const windows = process.platform === 'win32';
const binaryName = windows ? 'vp.exe' : 'vp';
const sourceBin = join(process.env.VP_HOME, 'current/bin');
const split = process.argv.includes('--split');
assert.ok(!split || !windows, 'The split fixture requires XDG-isolated config and state roots.');
const user = mkdtempSync(join(process.cwd(), 'legacy-user-'));
const data = join(user, split ? 'data' : '.vite-plus');
const bin = split ? join(user, 'shared-bin') : join(data, 'bin');
const cache = split ? join(user, 'cache') : join(data, 'cache');
const config = split ? join(user, '.config/vite-plus') : data;
const state = split ? join(user, '.local/state/vite-plus') : data;
const env = {
  ...process.env,
  HOME: user,
  USERPROFILE: user,
  ZDOTDIR: user,
  XDG_CONFIG_HOME: join(user, '.config'),
  XDG_DATA_HOME: join(user, '.local/share'),
  XDG_STATE_HOME: join(user, '.local/state'),
  APPDATA: join(user, 'AppData/Roaming'),
  LOCALAPPDATA: join(user, 'AppData/Local'),
  VP_SELF_SETUP_NO_MODIFY_PATH: '1',
  VP_SKIP_DEPS_INSTALL: '1',
  CI: '1',
  NO_COLOR: '1',
};
for (const key of ['VP_HOME', 'VP_DATA_DIR', 'VP_CACHE_DIR', 'VP_BIN_DIR']) delete env[key];
if (split) {
  Object.assign(env, { VP_DATA_DIR: data, VP_CACHE_DIR: cache, VP_BIN_DIR: bin });
} else {
  env.VP_HOME = data;
}

function write(path, contents) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, contents);
}

function createVersion(name, active = false) {
  const directory = join(data, name);
  const packageVersion = name.split('+')[0];
  const lockfile = `lockfileVersion: '9.0'\nimporters:\n  .:\n    dependencies:\n      vite-plus:\n        specifier: ${packageVersion}\n        version: ${packageVersion}\n`;
  write(join(directory, 'bin/vp-install-lock.yaml'), lockfile);
  write(
    join(directory, 'node_modules/.vp-install-complete'),
    createHash('sha256').update(lockfile).digest(),
  );
  write(
    join(directory, 'package.json'),
    JSON.stringify({
      name: 'vp-global',
      version: packageVersion,
      private: true,
      dependencies: { 'vite-plus': packageVersion },
    }),
  );
  write(
    join(directory, 'node_modules/vite-plus/package.json'),
    JSON.stringify({ name: 'vite-plus', version: packageVersion }),
  );
  if (active) {
    copyFileSync(join(sourceBin, binaryName), join(directory, 'bin', binaryName));
    if (windows) copyFileSync(join(sourceBin, 'vp-shim.exe'), join(directory, 'bin/vp-shim.exe'));
  } else {
    write(join(directory, 'bin', binaryName), 'Historical binaries must not be executed.\n');
    write(join(directory, 'bin/.vp-setup-complete'), '');
  }
  return directory;
}

function run(binary, args) {
  const result = spawnSync(binary, args, { env, encoding: 'utf8', timeout: 30000 });
  assert.equal(result.status, 0, `${result.error ?? ''}\n${result.stdout}\n${result.stderr}`);
  return result.stdout + result.stderr;
}

try {
  const old = createVersion('1.4.0');
  const forced = createVersion('1.4.0+force.123.456');
  const current = createVersion('1.5.0', true);
  const active = join(current, 'bin', binaryName);
  symlinkSync(current, join(data, 'current'), windows ? 'junction' : 'dir');
  const retained = [
    [join(data, 'personal.txt'), 'personal data'],
    [
      join(data, 'js_runtime/node/22.0.0', windows ? 'node.exe' : 'bin/node'),
      'legacy runtime without an ownership receipt',
    ],
    [join(data, 'package_manager/pnpm/10.0.0/pnpm'), 'legacy package manager'],
    [join(cache, 'downloads'), 'legacy cache'],
    [join(config, 'config.json'), JSON.stringify({ nodeShimMode: 'system_first' })],
    [join(state, 'legacy-state'), 'legacy state'],
    [join(bin, 'unrelated-tool'), 'another tool'],
  ];
  for (const [path, contents] of retained) write(path, contents);
  const incomplete = createVersion('1.3.0');
  rmSync(join(incomplete, 'node_modules/.vp-install-complete'));
  const unlocked = createVersion('1.3.1');
  write(join(unlocked, 'node_modules/.vp-install-complete'), 'unlocked');
  const mismatched = createVersion('1.3.2');
  write(join(mismatched, 'bin/vp-install-lock.yaml'), 'different lockfile');

  // This is the final command issued by the 1.4.0 upgrader after switching current.
  run(active, ['env', 'setup', '--refresh']);
  for (const root of new Set([data, cache, config, state])) {
    assert.equal(existsSync(join(root, '.vp-owned')), false, `Upgrade claimed ${root}`);
  }
  assert.equal(existsSync(join(current, 'bin/.vp-setup-complete')), true);
  console.log('Upgrade refresh completes without claiming existing roots.');
  const preservedContents = retained.map(([path]) => [path, readFileSync(path)]);
  const profile = join(env.XDG_CONFIG_HOME, 'nushell/config.nu');
  write(
    profile,
    `# personal config\n# Vite+ bin\nsource "${config.replaceAll('\\', '/')}/env.nu"\n`,
  );
  const output = run(active, ['implode', '--yes']);
  for (const directory of [old, forced, current])
    assert.equal(existsSync(directory), false, directory);
  for (const path of [
    join(data, 'current'),
    join(bin, binaryName),
    join(data, 'fallback-bin', windows ? 'node.exe' : 'node'),
  ]) {
    assert.equal(lstatSync(path, { throwIfNoEntry: false }), undefined);
  }
  assert.equal(readFileSync(profile, 'utf8'), '# personal config\n');
  console.log('Verified current and historical CLI versions, links, and shims are removed.');
  for (const [path, contents] of preservedContents)
    assert.deepEqual(readFileSync(path), contents, path);
  for (const directory of [incomplete, unlocked, mismatched])
    assert.equal(existsSync(directory), true);
  assert.match(output, /preserved to protect unverified data/);
  for (const name of [
    'personal.txt',
    'js_runtime',
    'package_manager',
    'cache',
    'config.json',
    'legacy-state',
    '1.3.0',
    '1.3.1',
    '1.3.2',
  ])
    assert.ok(output.includes(name), output);
  console.log(
    'Unverified runtimes, package managers, cache, config, and incomplete versions remain and are reported.',
  );
  console.log(
    'Personal files and unrelated tools are unchanged; only the matching shell entry is removed.',
  );
} finally {
  rmSync(user, { recursive: true, force: true, maxRetries: 10, retryDelay: 100 });
}
