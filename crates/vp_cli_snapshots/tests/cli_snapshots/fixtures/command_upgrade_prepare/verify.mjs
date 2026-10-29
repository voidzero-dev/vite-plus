import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import { createHash } from 'node:crypto';
import { once } from 'node:events';
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  realpathSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs';
import { createServer } from 'node:http';
import { dirname, join, resolve } from 'node:path';
import { promisify } from 'node:util';

import { createArchive } from './archive.mjs';

const exec = promisify(execFile);
const mode = process.argv[2];
const upgrading = mode.startsWith('upgrade');
const failure = mode.endsWith('failure');
const windows = process.platform === 'win32';
const binaryName = windows ? 'vp.exe' : 'vp';
const runnerHome = process.env.VP_HOME;
const cli = realpathSync(join(runnerHome, 'current/node_modules/vite-plus'));
const root = resolve(cli, '../..');
// Optional binaries and target pin also allow a local run with two distinct release builds.
const sourceBinary = process.argv[3] ?? join(runnerHome, 'current/bin', binaryName);
const targetBinary = process.argv[4] ?? join(runnerHome, 'current/bin', binaryName);
const nodeVersion = process.argv[5] ?? readFileSync(join(root, '.node-version'), 'utf8').trim();
const version = JSON.parse(readFileSync(join(cli, 'package.json'), 'utf8')).version;
const pnpmVersion = /export const INSTALL_PNPM_VERSION = '([^']+)';/.exec(
  readFileSync(join(cli, 'generate-install-lockfile.ts'), 'utf8'),
)[1];
const home = resolve('upgrade-home');
const user = resolve('upgrade-user');
const old = join(home, upgrading ? version : 'old-version');
const target = join(home, upgrading ? 'payload' : version);
const binary = join(target, 'bin', binaryName);
const lockfile = "lockfileVersion: '9.0'\n";
const retained = new Map([
  [join(home, '.previous-version'), 'rollback-sentinel'],
  [join(home, 'config.json'), '{"nodeShimMode":"system_first"}'],
  [join(home, 'env'), '# Existing environment\n'],
  [join(user, '.bashrc'), '# Existing shell configuration\n'],
]);

function write(file, content) {
  mkdirSync(dirname(file), { recursive: true });
  writeFileSync(file, content);
}

for (const [file, content] of retained) write(file, content);
write(join(old, 'bin', binaryName), 'The current binary must not be replaced.\n');
mkdirSync(join(target, 'bin'), { recursive: true });
copyFileSync(targetBinary, binary);
if (windows) {
  copyFileSync(join(runnerHome, 'current/bin/vp-shim.exe'), join(target, 'bin/vp-shim.exe'));
}
write(join(target, 'bin/vp-install-lock.yaml'), lockfile);
symlinkSync(old, join(home, 'current'), windows ? 'junction' : 'dir');

// Reject unexpected lookups locally: this fixture must use only its private cache.
const requests = [];
let upgradeArchive;
let registry;
if (upgrading) {
  copyFileSync(sourceBinary, join(old, 'bin', binaryName));
  write(join(old, 'bin/.vp-setup-complete'), '');
  write(join(old, 'package.json'), JSON.stringify({ name: 'vp-global', version }));
  const payload = {
    'package/package.json': JSON.stringify({ version, vpInstallLockfile: true }),
    [`package/${binaryName}`]: readFileSync(binary),
    'package/vp-install-lock.yaml': lockfile,
  };
  if (windows) payload['package/vp-shim.exe'] = readFileSync(join(target, 'bin/vp-shim.exe'));
  upgradeArchive = createArchive(payload);
}
const server = createServer((request, response) => {
  if (upgrading && request.url === '/platform.tgz') {
    response.end(upgradeArchive);
    return;
  }
  if (
    upgrading &&
    (request.url === `/vite-plus/${version}` ||
      request.url.startsWith('/@voidzero-dev/vite-plus-cli-'))
  ) {
    response.setHeader('Content-Type', 'application/json');
    response.end(
      JSON.stringify({
        version,
        vpInstallLockfile: true,
        dist: {
          tarball: `${registry}/platform.tgz`,
          integrity: `sha512-${createHash('sha512').update(upgradeArchive).digest('base64')}`,
          attestations: { provenance: { predicateType: 'https://slsa.dev/provenance/v1' } },
        },
      }),
    );
    return;
  }
  requests.push(request.url);
  response.writeHead(500).end('Unexpected network request');
});
server.listen(0, '127.0.0.1');
await once(server, 'listening');
registry = `http://127.0.0.1:${server.address().port}`;
const env = {
  ...process.env,
  HOME: user,
  USERPROFILE: user,
  VP_HOME: home,
  VP_NODE_DIST_MIRROR: registry,
  NPM_CONFIG_REGISTRY: registry,
  npm_config_registry: registry,
  VP_SELF_SETUP_NO_MODIFY_PATH: '1',
  VP_NO_UPDATE_CHECK: '1',
  CI: '1',
  NO_COLOR: '1',
};
for (const name of [
  'VP_UPGRADE_PREPARE',
  'VP_SELF_SETUP_SUPPORT_CHECK',
  'VP_SELF_SETUP_SHELL',
  'VP_SELF_SETUP_REPLACE_EXISTING',
  'VP_DATA_DIR',
  'VP_CACHE_DIR',
  'VP_BIN_DIR',
  'VP_SKIP_DEPS_INSTALL',
  'VP_NODE_VERSION',
  'VP_SETUP_NODE_VERSION',
  'VP_SETUP_NODE_VERSION_FILE',
]) {
  delete env[name];
}

async function run(args, additions = {}, executable = binary) {
  const pending = exec(executable, args, {
    env: { ...env, ...additions },
    timeout: 30000,
    maxBuffer: 1024 * 1024,
  });
  try {
    const result = await pending;
    return { ...result, code: 0, pid: pending.child.pid };
  } catch (error) {
    assert.equal(error.signal, null, error.message);
    assert.equal(typeof error.code, 'number', error.message);
    return { ...error, pid: pending.child.pid };
  }
}

function assertUnchanged() {
  assert.equal(realpathSync(join(home, 'current')), realpathSync(old));
  for (const [file, content] of retained) assert.equal(readFileSync(file, 'utf8'), content);
  assert.equal(existsSync(join(target, 'bin/.vp-setup-complete')), false);
  assert.equal(existsSync(join(home, 'bin')), false);
  assert.equal(existsSync(join(user, '.zshenv')), false);
  assert.equal(existsSync(join(home, '.vp-owned')), false);
}

try {
  if (mode === 'protocol') {
    for (const [request, response] of [
      ['1', 'vite-plus-self-setup-v1\n'],
      ['upgrade-prepare-v1', 'vite-plus-upgrade-prepare-v1\n'],
    ]) {
      const result = await run(['--help'], { VP_SELF_SETUP_SUPPORT_CHECK: request });
      assert.equal(result.code, 0, result.stderr);
      assert.equal(result.stdout, response);
      assert.equal(result.stderr, '');
      assertUnchanged();
    }
    console.log('Both capability probes return their exact protocol without setup.');
    for (const [args, protocol] of [
      [[], 'unsupported'],
      [['--help'], 'upgrade-prepare-v1'],
    ]) {
      const result = await run(args, { VP_UPGRADE_PREPARE: protocol });
      assert.equal(result.code, 1, result.stderr);
      assertUnchanged();
      assert.equal(existsSync(join(target, 'package.json')), false);
    }
    console.log('Unsupported preparation requests fail before installation or activation.');
  } else {
    const nodePath = windows ? 'node.exe' : 'bin/node';
    const cachedNode = join(home, 'js_runtime/node', nodeVersion, nodePath);
    mkdirSync(dirname(cachedNode), { recursive: true });
    copyFileSync(join(runnerHome, 'js_runtime/node', nodeVersion, nodePath), cachedNode);
    const pnpm = join(home, 'package_manager/pnpm', pnpmVersion, 'pnpm');
    write(
      join(pnpm, 'package.json'),
      JSON.stringify({ version: pnpmVersion, engines: { node: '*' } }),
    );
    for (const shim of ['pnpm', 'pnpm.cmd', 'pnpm.ps1']) write(join(pnpm, 'bin', shim), 'unused');
    write(
      join(pnpm, 'bin/pnpm.cjs'),
      `const assert = require('node:assert/strict');
const fs = require('node:fs');
assert.equal(fs.realpathSync(process.execPath), fs.realpathSync(${JSON.stringify(cachedNode)}));
assert.equal(process.version, ${JSON.stringify(`v${nodeVersion}`)});
assert.equal(process.env.VP_UPGRADE_PREPARE, undefined);
assert.equal(fs.realpathSync(${JSON.stringify(join(home, 'current'))}), fs.realpathSync(${JSON.stringify(old)}));
assert.equal(fs.readFileSync(${JSON.stringify(join(home, '.previous-version'))}, 'utf8'), 'rollback-sentinel');
assert.equal(process.env.npm_config_registry, ${JSON.stringify(registry)});
if (${upgrading}) assert.equal(process.env.NPM_CONFIG_REGISTRY, ${JSON.stringify(registry)});
assert.equal(process.argv.includes('--registry'), false);
assert.ok(process.argv.includes('--frozen-lockfile'));
assert.equal(fs.readFileSync('pnpm-lock.yaml', 'utf8'), ${JSON.stringify(lockfile)});
const manifest = JSON.parse(fs.readFileSync('package.json', 'utf8'));
assert.equal(manifest.dependencies['vite-plus'], ${JSON.stringify(version)});
assert.equal(manifest.packageManager, ${JSON.stringify(`pnpm@${pnpmVersion}`)});
fs.appendFileSync('install-count', ${JSON.stringify('installed\n')});
fs.appendFileSync(${JSON.stringify(join(home, 'install-attempts'))}, JSON.stringify({ parent: process.ppid, directory: process.cwd() }) + ${JSON.stringify('\n')});
fs.mkdirSync('node_modules/vite-plus', { recursive: true });
fs.writeFileSync('node_modules/vite-plus/package.json', JSON.stringify({name: 'vite-plus', version: ${JSON.stringify(version)}}));
console.log('Fixture pnpm stdout');
console.error('Fixture pnpm stderr');
if (${failure}) process.exit(17);
`,
    );
    const overrides = {
      VP_NODE_VERSION: '20.18.0',
      VP_SETUP_NODE_VERSION: '20.18.0',
      VP_SETUP_NODE_VERSION_FILE: join(home, 'selected-node'),
      VP_SKIP_DEPS_INSTALL: '1',
      VP_VERSION: 'wrong-initial-install-version',
      VP_SELF_SETUP_SHELL: 'sh',
      NPM_CONFIG_REGISTRY: `${registry}/wrong-uppercase-registry`,
    };
    // Initial-install shell mode intercepts normal commands before the upgrade dispatcher.
    if (upgrading) delete overrides.VP_SELF_SETUP_SHELL;
    const result = upgrading
      ? await run(
          ['upgrade', version, '--force', '--silent', '--registry', registry],
          overrides,
          join(old, 'bin', binaryName),
        )
      : await run(['--silent'], { ...overrides, VP_UPGRADE_PREPARE: 'upgrade-prepare-v1' });
    assert.equal(result.code, failure ? 1 : 0, result.stderr);
    if (!upgrading) assert.equal(result.stdout, '');
    if (!failure) assert.equal(result.stderr, '');
    const attempts = readFileSync(join(home, 'install-attempts'), 'utf8')
      .trim()
      .split('\n')
      .map(JSON.parse);
    assert.equal(attempts.length, 1);
    const installed = attempts[0].directory;
    assert.equal(existsSync(join(home, 'selected-node')), false);
    console.log('Preparation uses the target Node and pnpm pins despite initial-install overrides.');
    if (!upgrading || failure) {
      assertUnchanged();
      console.log('Current, rollback history, preferences, shell files, and setup marker are unchanged.');
    }
    const receipt = join(installed, 'node_modules/.vp-install-complete');
    if (failure) {
      assert.equal(existsSync(receipt), false);
      const log = readFileSync(join(home, 'upgrade.log'), 'utf8');
      assert.ok(log.includes('Fixture pnpm stdout'));
      assert.ok(log.includes('Fixture pnpm stderr'));
      console.log('Failed preparation records both output streams and leaves no completion receipt.');
      if (upgrading) {
        assert.equal(existsSync(installed), false);
        console.log('Upgrade removes the failed staged version and preserves the active version.');
      }
    } else {
      assert.deepEqual(readFileSync(receipt), createHash('sha256').update(lockfile).digest());
      if (upgrading) {
        assert.notEqual(attempts[0].parent, result.pid);
        assert.equal(realpathSync(join(home, 'current')), realpathSync(installed));
        assert.equal(readFileSync(join(home, '.previous-version'), 'utf8'), version);
        assert.ok(readdirSync(home).some((name) => name.startsWith(`${version}+force.`)));
        console.log('The target process prepares a separate version before forced-upgrade activation.');
      } else {
        rmSync(join(home, 'current'));
        symlinkSync(target, join(home, 'current'), windows ? 'junction' : 'dir');
        const refreshed = await run(['env', 'setup', '--refresh']);
        assert.equal(refreshed.code, 0, refreshed.stderr);
        assert.equal(readFileSync(join(home, '.previous-version'), 'utf8'), 'rollback-sentinel');
      }
      assert.equal(existsSync(join(installed, 'bin/.vp-setup-complete')), true);
      assert.equal(readFileSync(join(installed, 'install-count'), 'utf8'), 'installed\n');
      console.log('Post-activation setup reuses the receipt without installing dependencies again.');
    }
  }
  assert.deepEqual(requests, []);
  console.log(
    upgrading
      ? 'All release requests stayed local; no runtime downloads occurred.'
      : 'No registry or Node mirror requests were made.',
  );
} finally {
  server.close();
  server.closeAllConnections();
  for (const directory of [home, user]) {
    rmSync(directory, { recursive: true, force: true, maxRetries: 10, retryDelay: 100 });
  }
}
