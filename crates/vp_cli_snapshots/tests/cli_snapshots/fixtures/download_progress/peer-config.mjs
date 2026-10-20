import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import { createHash } from 'node:crypto';
import { once } from 'node:events';
import {
  copyFileSync,
  cpSync,
  existsSync,
  mkdirSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs';
import { createServer } from 'node:http';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { promisify } from 'node:util';

import { createArchive } from './archive.mjs';

const exec = promisify(execFile);
const runnerHome = process.env.VP_HOME;
const cli = join(runnerHome, 'current/node_modules/vite-plus');
const { generateInstallLockfile, installManifest, INSTALL_PNPM_VERSION } = await import(
  pathToFileURL(join(cli, 'generate-install-lockfile.ts')).href
);
const version = JSON.parse(readFileSync(join(cli, 'package.json'), 'utf8')).version;

// Only the pinned pnpm distribution uses the runner's cached registry. Real pnpm
// is necessary to exercise configuration precedence and frozen-lockfile checks.
await exec(join(runnerHome, 'bin/vp'), ['env', 'install', `pnpm@${INSTALL_PNPM_VERSION}`]);
const pnpmRoot = join(runnerHome, 'package_manager/pnpm', INSTALL_PNPM_VERSION);
const pnpm = join(pnpmRoot, 'pnpm/bin/pnpm.cjs');
assert.equal(
  (await exec(process.execPath, [pnpm, '--version'])).stdout.trim(),
  INSTALL_PNPM_VERSION,
);

const root = resolve('peer-config');
const home = join(root, 'home');
const user = join(root, 'user');
const external = join(root, 'external');
for (const directory of [user, external]) mkdirSync(directory, { recursive: true });

const requests = [];
const packages = new Map(
  [
    { name: 'vite-plus', version, peerDependencies: { 'fixture-peer': '1.0.0' } },
    { name: 'fixture-peer', version: '1.0.0' },
  ].map((manifest) => {
    const archive = createArchive({ 'package/package.json': JSON.stringify(manifest) });
    return [manifest.name, { manifest, archive }];
  }),
);
let registry;
const server = createServer((request, response) => {
  requests.push({ url: request.url, authorization: request.headers.authorization });
  if (request.headers.authorization !== 'Bearer fixture-token') {
    response.writeHead(401).end();
    return;
  }
  const name = request.url.slice(1).split('/')[0];
  const pkg = packages.get(name);
  if (!pkg) {
    response.writeHead(404).end();
    return;
  }
  if (request.url.endsWith('.tgz')) {
    response.end(pkg.archive);
    return;
  }
  const manifest = {
    ...pkg.manifest,
    dist: {
      tarball: `${registry}/${name}/-/${name}-${pkg.manifest.version}.tgz`,
      integrity: `sha512-${createHash('sha512').update(pkg.archive).digest('base64')}`,
    },
  };
  response.setHeader('Content-Type', 'application/json');
  response.end(
    JSON.stringify({
      name,
      'dist-tags': { latest: manifest.version },
      versions: { [manifest.version]: manifest },
    }),
  );
});
server.listen(0, '127.0.0.1');
await once(server, 'listening');
registry = `http://127.0.0.1:${server.address().port}`;

const userconfig = join(user, '.npmrc');
const configuration = [
  'auto-install-peers=false',
  `registry=${registry}/`,
  `//127.0.0.1:${server.address().port}/:_authToken=fixture-token`,
  `store-dir=${join(user, 'store')}`,
  `cache-dir=${join(user, 'cache')}`,
  '',
].join('\n');
writeFileSync(userconfig, configuration);
const env = {
  ...process.env,
  HOME: user,
  CI: 'true',
  npm_config_registry: undefined,
  NPM_CONFIG_REGISTRY: undefined,
  npm_config_userconfig: userconfig,
  NPM_CONFIG_USERCONFIG: userconfig,
};
const runPnpm = (cwd, args, environment = env) =>
  exec(process.execPath, [pnpm, ...args], { cwd, env: environment });

try {
  const lockfile = await generateInstallLockfile(version, registry, async (cwd, args) => {
    // The command-line pin must beat both a user npmrc and environment overrides.
    await runPnpm(cwd, args, { ...env, npm_config_auto_install_peers: 'false' });
  });
  assert.match(lockfile, /autoInstallPeers: true/);
  assert.match(lockfile, /fixture-peer@1\.0\.0/);
  console.log('The generator pins autoInstallPeers=true despite user and environment settings.');

  const control = join(root, 'control');
  mkdirSync(control);
  writeFileSync(join(control, 'package.json'), JSON.stringify(installManifest(version)));
  writeFileSync(join(control, '.npmrc'), 'minimum-release-age=0\n');
  writeFileSync(join(control, 'pnpm-lock.yaml'), lockfile);
  await assert.rejects(
    runPnpm(control, ['install', '--ignore-workspace', '--frozen-lockfile']),
    (error) =>
      error.code === 1 && /ERR_PNPM_LOCKFILE_CONFIG_MISMATCH/.test(error.stdout + error.stderr),
  );
  console.log('Without the installer override, real pnpm rejects the user configuration.');

  const nodeBin = join(home, 'js_runtime/node', process.versions.node, 'bin');
  mkdirSync(nodeBin, { recursive: true });
  symlinkSync(process.execPath, join(nodeBin, 'node'));
  cpSync(pnpmRoot, join(home, 'package_manager/pnpm', INSTALL_PNPM_VERSION), { recursive: true });
  copyFileSync(join(runnerHome, 'bin/vp'), join(external, 'vp'));
  writeFileSync(join(external, 'vp-install-lock.yaml'), lockfile);
  const before = requests.length;
  await exec(join(external, 'vp'), [], {
    cwd: root,
    env: {
      ...env,
      VP_HOME: home,
      VP_SELF_SETUP_SHELL: 'sh',
      VP_SELF_SETUP_NO_MODIFY_PATH: '1',
      VP_SKIP_DEPS_INSTALL: '',
      VP_NODE_MANAGER: 'no',
      VP_PM_MANAGER: 'no',
      VP_SETUP_NODE_VERSION: process.versions.node,
      npm_config_auto_install_peers: 'false',
    },
  });
  const current = join(home, 'current');
  assert.equal(readFileSync(join(current, 'pnpm-lock.yaml'), 'utf8'), lockfile);
  assert.equal(existsSync(join(current, 'node_modules/.vp-install-complete')), true);
  assert.equal(existsSync(join(current, 'node_modules/vite-plus/package.json')), true);
  assert.ok(requests.slice(before).some(({ url }) => url.endsWith(`vite-plus-${version}.tgz`)));
  assert.ok(requests.slice(before).some(({ url }) => url.endsWith('fixture-peer-1.0.0.tgz')));
  assert.ok(requests.every(({ authorization }) => authorization === 'Bearer fixture-token'));
  assert.equal(readFileSync(userconfig, 'utf8'), configuration);
  console.log(
    'Frozen setup installs the CLI and its peer using the user registry and authentication.',
  );
  console.log('The published lockfile and user configuration remain unchanged.');
} finally {
  server.close();
  server.closeAllConnections();
  rmSync(root, { recursive: true, force: true });
}
