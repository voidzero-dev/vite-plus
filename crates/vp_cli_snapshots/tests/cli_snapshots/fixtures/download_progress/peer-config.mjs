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
const external = join(root, 'external');
mkdirSync(external, { recursive: true });
const conflicts = [
  { option: 'auto-install-peers', value: 'false', setting: 'autoInstallPeers' },
  { option: 'peers-suffix-max-length', value: '100', setting: 'peersSuffixMaxLength' },
  { option: 'exclude-links-from-lockfile', value: 'true', setting: 'excludeLinksFromLockfile' },
  { option: 'dedupe-peers', value: 'true', setting: 'dedupePeers' },
  { option: 'inject-workspace-packages', value: 'true', setting: 'injectWorkspacePackages' },
];

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

function configureUser(user, settings = []) {
  mkdirSync(user, { recursive: true });
  const userconfig = join(user, '.npmrc');
  const configuration = [
    `registry=${registry}/`,
    `//127.0.0.1:${server.address().port}/:_authToken=fixture-token`,
    `store-dir=${join(user, 'store')}`,
    `cache-dir=${join(user, 'cache')}`,
    ...settings.map(({ option, value }) => `${option}=${value}`),
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
  // Keep the test independent of the host's values for these settings.
  for (const { option } of conflicts) {
    const key = `npm_config_${option.replaceAll('-', '_')}`;
    delete env[key];
    delete env[key.toUpperCase()];
  }
  return { userconfig, configuration, env };
}

function conflictEnvironment(settings) {
  return Object.fromEntries(
    settings.flatMap(({ option, value }) => {
      const key = `npm_config_${option.replaceAll('-', '_')}`;
      return [
        [key, value],
        [key.toUpperCase(), value],
      ];
    }),
  );
}

const runPnpm = (cwd, args, env) => exec(process.execPath, [pnpm, ...args], { cwd, env });

try {
  const generationUser = join(root, 'generation-user');
  const defaults = configureUser(generationUser);
  const lockfile = await generateInstallLockfile(
    version,
    registry,
    async (cwd, args, overrides) => {
      await runPnpm(cwd, args, { ...defaults.env, ...overrides });
    },
  );
  assert.match(lockfile, /autoInstallPeers: true/);
  assert.match(lockfile, /fixture-peer@1\.0\.0/);
  // pnpm omits its numeric default of 1000; a CLI string would be recorded as '1000'.
  assert.doesNotMatch(lockfile, /peersSuffixMaxLength:/);
  const conflictingGeneration = configureUser(generationUser, conflicts);
  assert.equal(
    await generateInstallLockfile(version, registry, async (cwd, args, overrides) => {
      await runPnpm(cwd, args, {
        ...conflictingGeneration.env,
        ...conflictEnvironment(conflicts),
        ...overrides,
      });
    }),
    lockfile,
  );
  assert.equal(
    readFileSync(conflictingGeneration.userconfig, 'utf8'),
    conflictingGeneration.configuration,
  );
  console.log(
    'The generator pins all frozen-lockfile settings despite user and environment values.',
  );

  copyFileSync(join(runnerHome, 'bin/vp'), join(external, 'vp'));
  writeFileSync(join(external, 'vp-install-lock.yaml'), lockfile);
  for (const conflict of conflicts) {
    const directory = join(root, conflict.option);
    const home = join(directory, 'home');
    const { userconfig, configuration, env } = configureUser(join(directory, 'user'), [conflict]);
    const control = join(directory, 'control');
    mkdirSync(control);
    writeFileSync(join(control, 'package.json'), JSON.stringify(installManifest(version)));
    writeFileSync(join(control, '.npmrc'), 'minimum-release-age=0\n');
    writeFileSync(join(control, 'pnpm-lock.yaml'), lockfile);
    await assert.rejects(
      runPnpm(control, ['install', '--ignore-workspace', '--frozen-lockfile'], env),
      (error) =>
        error.code === 1 &&
        /ERR_PNPM_LOCKFILE_CONFIG_MISMATCH/.test(error.stdout + error.stderr) &&
        (error.stdout + error.stderr).includes(`settings.${conflict.setting}`),
    );
    console.log(`Without the installer override, real pnpm rejects ${conflict.option}.`);

    const nodeBin = join(home, 'js_runtime/node', process.versions.node, 'bin');
    mkdirSync(nodeBin, { recursive: true });
    symlinkSync(process.execPath, join(nodeBin, 'node'));
    cpSync(pnpmRoot, join(home, 'package_manager/pnpm', INSTALL_PNPM_VERSION), { recursive: true });
    const before = requests.length;
    await exec(join(external, 'vp'), [], {
      cwd: directory,
      env: {
        ...env,
        ...conflictEnvironment([conflict]),
        VP_HOME: home,
        VP_SELF_SETUP_SHELL: 'sh',
        VP_SELF_SETUP_NO_MODIFY_PATH: '1',
        VP_SKIP_DEPS_INSTALL: '',
        VP_NODE_MANAGER: 'no',
        VP_PM_MANAGER: 'no',
        VP_SETUP_NODE_VERSION: process.versions.node,
      },
    }).catch((error) => {
      const log = join(home, 'upgrade.log');
      if (existsSync(log)) error.message += `\n${readFileSync(log, 'utf8')}`;
      throw error;
    });
    const current = join(home, 'current');
    assert.equal(readFileSync(join(current, 'pnpm-lock.yaml'), 'utf8'), lockfile);
    assert.equal(existsSync(join(current, 'node_modules/.vp-install-complete')), true);
    assert.equal(existsSync(join(current, 'node_modules/vite-plus/package.json')), true);
    assert.ok(requests.slice(before).some(({ url }) => url.endsWith(`vite-plus-${version}.tgz`)));
    assert.ok(requests.slice(before).some(({ url }) => url.endsWith('fixture-peer-1.0.0.tgz')));
    assert.equal(readFileSync(userconfig, 'utf8'), configuration);
    console.log(`Frozen setup overrides ${conflict.option} in user and environment configuration.`);
  }
  assert.ok(requests.every(({ authorization }) => authorization === 'Bearer fixture-token'));
  console.log(
    'All installs fetch the CLI and its peer using the user registry and authentication.',
  );
  console.log('The published lockfile and user configuration remain unchanged.');
} finally {
  server.close();
  server.closeAllConnections();
  rmSync(root, { recursive: true, force: true });
}
