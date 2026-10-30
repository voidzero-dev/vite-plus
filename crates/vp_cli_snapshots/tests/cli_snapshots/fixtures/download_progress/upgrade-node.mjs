import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import { createHash } from 'node:crypto';
import { once } from 'node:events';
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
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
const legacy = mode === 'legacy';
const incompatible = mode === 'incompatible';
const failure = mode === 'failure';
const windows = process.platform === 'win32';
const binaryName = windows ? 'vp.exe' : 'vp';
const runnerHome = process.env.VP_HOME;
const cli = realpathSync(join(runnerHome, 'current/node_modules/vite-plus'));
const defaultNode = readFileSync(resolve(cli, '../../.node-version'), 'utf8').trim();
const targetNode = '22.23.4';
assert.notEqual(targetNode, defaultNode);
const expectedNode = legacy ? defaultNode : targetNode;
const version = JSON.parse(readFileSync(join(cli, 'package.json'), 'utf8')).version;
const pnpmVersion = /export const INSTALL_PNPM_VERSION = '([^']+)';/.exec(
  readFileSync(join(cli, 'generate-install-lockfile.ts'), 'utf8'),
)[1];
const home = resolve('upgrade-home');
const user = resolve('upgrade-user');
const current = join(home, 'current');
const old = join(home, version);
const source = join(old, 'bin', binaryName);
const previous = join(home, '.previous-version');
const attempts = join(home, 'install-attempts');
const lockfile = "lockfileVersion: '9.0'\n";

function write(file, contents) {
  mkdirSync(dirname(file), { recursive: true });
  writeFileSync(file, contents);
}

write(previous, 'rollback-sentinel');
write(join(old, 'bin/.vp-setup-complete'), '');
write(join(old, 'package.json'), JSON.stringify({ name: 'vp-global', version }));
copyFileSync(join(runnerHome, 'current/bin', binaryName), source);
symlinkSync(old, current, windows ? 'junction' : 'dir');
mkdirSync(user);

const payload = {
  'package/package.json': JSON.stringify({ version, vpInstallLockfile: true }),
  [`package/${binaryName}`]: readFileSync(source),
  'package/vp-install-lock.yaml': lockfile,
};
if (windows) {
  payload['package/vp-shim.exe'] = readFileSync(join(runnerHome, 'current/bin/vp-shim.exe'));
}
const archive = createArchive(payload);
const requests = [];
let registry;
const server = createServer((request, response) => {
  requests.push(request.url);
  if (request.url === '/platform.tgz') {
    response.end(archive);
    return;
  }
  const main = request.url === '/vite-plus/latest';
  const platform =
    request.url.startsWith('/@voidzero-dev/vite-plus-cli-') &&
    request.url.endsWith(`/${version}`);
  if (main || platform) {
    // Only the selected platform package determines the installation runtime.
    const nodeMetadata = {};
    if (main) nodeMetadata.vpNodeVersion = defaultNode;
    else if (!legacy) nodeMetadata.vpNodeVersion = targetNode;
    response.setHeader('Content-Type', 'application/json');
    response.end(JSON.stringify({
      version,
      vpInstallLockfile: true,
      ...nodeMetadata,
      dist: {
        tarball: `${registry}/platform.tgz`,
        integrity: `sha512-${createHash('sha512').update(archive).digest('base64')}`,
        attestations: { provenance: { predicateType: 'https://slsa.dev/provenance/v1' } },
      },
    }));
    return;
  }
  response.writeHead(404).end('Unexpected network request');
});
server.listen(0, '127.0.0.1');
await once(server, 'listening');
registry = `http://127.0.0.1:${server.address().port}`;

const nodePath = windows ? 'node.exe' : 'bin/node';
for (const nodeVersion of [defaultNode, targetNode]) {
  const destination = join(home, 'js_runtime/node', nodeVersion, nodePath);
  mkdirSync(dirname(destination), { recursive: true });
  copyFileSync(join(runnerHome, 'js_runtime/node', nodeVersion, nodePath), destination);
}
const pnpm = join(home, 'package_manager/pnpm', pnpmVersion, 'pnpm');
write(join(pnpm, 'package.json'), JSON.stringify({
  version: pnpmVersion,
  engines: { node: incompatible ? `>=${defaultNode}` : '*' },
}));
for (const shim of ['pnpm', 'pnpm.cmd', 'pnpm.ps1']) write(join(pnpm, 'bin', shim), 'unused');
write(join(pnpm, 'bin/pnpm.cjs'), `
const assert = require('node:assert/strict');
const fs = require('node:fs');
assert.equal(process.versions.node, ${JSON.stringify(expectedNode)});
assert.equal(fs.realpathSync(process.execPath), fs.realpathSync(${JSON.stringify(join(home, 'js_runtime/node', expectedNode, nodePath))}));
assert.equal(fs.realpathSync(${JSON.stringify(current)}), fs.realpathSync(${JSON.stringify(old)}));
assert.equal(fs.readFileSync(${JSON.stringify(previous)}, 'utf8'), 'rollback-sentinel');
assert.ok(process.argv.includes('--frozen-lockfile'));
assert.equal(fs.readFileSync('pnpm-lock.yaml', 'utf8'), ${JSON.stringify(lockfile)});
assert.equal(JSON.parse(fs.readFileSync('package.json')).dependencies['vite-plus'], ${JSON.stringify(version)});
fs.appendFileSync(${JSON.stringify(attempts)}, process.cwd() + '\\n');
if (${failure}) process.exit(17);
fs.mkdirSync('node_modules/vite-plus', { recursive: true });
fs.writeFileSync('node_modules/vite-plus/package.json', JSON.stringify({name: 'vite-plus', version: ${JSON.stringify(version)}}));
`);

const env = {
  ...process.env,
  HOME: user,
  USERPROFILE: user,
  VP_HOME: home,
  VP_NODE_DIST_MIRROR: registry,
  npm_config_registry: registry,
  NPM_CONFIG_REGISTRY: registry,
  VP_SELF_SETUP_NO_MODIFY_PATH: '1',
  VP_NO_UPDATE_CHECK: '1',
  CI: '1',
  NO_COLOR: '1',
};
for (const name of [
  'VP_SELF_SETUP_SHELL', 'VP_DATA_DIR', 'VP_CACHE_DIR', 'VP_BIN_DIR',
  'VP_SKIP_DEPS_INSTALL', 'VP_NODE_VERSION', 'VP_SETUP_NODE_VERSION', 'VP_SETUP_NODE_VERSION_FILE',
]) delete env[name];

try {
  const result = await exec(source, ['upgrade', '--force', '--silent', '--registry', registry], {
    env,
    timeout: 30000,
  }).then(
    (result) => ({ ...result, code: 0 }),
    (error) => {
      assert.equal(error.signal, null, error.message);
      return error;
    },
  );
  assert.equal(result.code, failure || incompatible ? 1 : 0, result.stderr);
  if (incompatible) {
    assert.ok(result.stderr.includes(targetNode), result.stderr);
    assert.equal(existsSync(attempts), false);
    console.log('An incompatible target Node pin fails without falling back or starting pnpm.');
  } else {
    const directories = readFileSync(attempts, 'utf8').trim().split('\n');
    assert.equal(directories.length, 1);
    const installed = directories[0];
    console.log(legacy
      ? 'Missing platform metadata preserves the running binary\'s embedded Node default.'
      : 'Upgrade installs dependencies with the target metadata\'s Node pin before activation.');
    if (failure) {
      assert.equal(existsSync(installed), false);
      console.log('A failed dependency install removes the staged version.');
    } else {
      assert.equal(realpathSync(current), realpathSync(installed));
      assert.equal(readFileSync(previous, 'utf8'), version);
      assert.deepEqual(readFileSync(join(installed, 'node_modules/.vp-install-complete')),
        createHash('sha256').update(lockfile).digest());
      console.log('Activation follows successful installation; setup reuses the completion receipt.');
    }
  }
  if (failure || incompatible) {
    assert.equal(realpathSync(current), realpathSync(old));
    assert.equal(readFileSync(previous, 'utf8'), 'rollback-sentinel');
    console.log('The current version and rollback record remain unchanged.');
  }
  assert.equal(requests.length, 3);
  assert.equal(requests[0], '/vite-plus/latest');
  assert.ok(requests[1].endsWith(`/${version}`));
  assert.equal(requests[2], '/platform.tgz');
  console.log('Only the two metadata requests and platform download occur; all stay local.');
} finally {
  server.close();
  server.closeAllConnections();
  for (const directory of [home, user]) {
    rmSync(directory, { recursive: true, force: true, maxRetries: 10, retryDelay: 100 });
  }
}
