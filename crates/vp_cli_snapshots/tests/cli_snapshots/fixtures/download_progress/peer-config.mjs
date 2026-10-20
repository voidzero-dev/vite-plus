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
  readlinkSync,
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
const bypass = process.argv.includes('bypass');
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
const disabledLockfile = { option: 'lockfile', value: 'false' };

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
let upgradeArchive;
let upgradeIntegrity;
const server = createServer((request, response) => {
  // Native upgrade metadata is separate from the authenticated pnpm fixture registry.
  if (upgradeArchive) {
    if (request.url === '/platform.tgz') {
      response.end(upgradeArchive);
      return;
    }
    if (request.url === '/index.json') {
      response.setHeader('Content-Type', 'application/json');
      response.end(JSON.stringify([{ version: process.version, lts: 'Fixture' }]));
      return;
    }
    if (
      request.url === `/vite-plus/${version}` ||
      request.url.startsWith('/@voidzero-dev/vite-plus-cli-')
    ) {
      response.setHeader('Content-Type', 'application/json');
      response.end(
        JSON.stringify({
          version,
          vpInstallLockfile: true,
          dist: {
            tarball: `${registry}/platform.tgz`,
            integrity: upgradeIntegrity,
            attestations: { provenance: { predicateType: 'https://slsa.dev/provenance/v1' } },
          },
        }),
      );
      return;
    }
  }
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
    VP_SETUP_SKIP_INSTALL_LOCKFILE: undefined,
  };
  // Keep the test independent of the host's values for these settings.
  for (const { option } of [...conflicts, disabledLockfile]) {
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

function runPnpm(cwd, args, env) {
  return exec(process.execPath, [pnpm, ...args], { cwd, env });
}

function assertDependencyDownloads(since) {
  const downloads = requests.slice(since);
  assert.ok(downloads.some(({ url }) => url.endsWith(`vite-plus-${version}.tgz`)));
  assert.ok(downloads.some(({ url }) => url.endsWith('fixture-peer-1.0.0.tgz')));
}

function createInstallation(name, settings = []) {
  const directory = join(root, name);
  const home = join(directory, 'home');
  const user = configureUser(join(directory, 'user'), settings);
  const nodeBin = join(home, 'js_runtime/node', process.versions.node, 'bin');
  mkdirSync(nodeBin, { recursive: true });
  symlinkSync(process.execPath, join(nodeBin, 'node'));
  cpSync(pnpmRoot, join(home, 'package_manager/pnpm', INSTALL_PNPM_VERSION), { recursive: true });
  return { directory, home, ...user };
}

async function runSetup(installation, overrides = {}, binary = join(external, 'vp'), args = []) {
  const { directory, home, env } = installation;
  return exec(binary, args, {
    cwd: directory,
    env: {
      ...env,
      VP_HOME: home,
      VP_NODE_DIST_MIRROR: registry,
      VP_SELF_SETUP_SHELL: args.length === 0 ? 'sh' : undefined,
      VP_SELF_SETUP_NO_MODIFY_PATH: '1',
      VP_SKIP_DEPS_INSTALL: '',
      VP_NODE_MANAGER: 'no',
      VP_PM_MANAGER: 'no',
      VP_SETUP_NODE_VERSION: process.versions.node,
      ...overrides,
    },
  }).catch((error) => {
    const log = join(home, 'upgrade.log');
    if (existsSync(log)) error.message += `\n${readFileSync(log, 'utf8')}`;
    throw error;
  });
}

async function testBypass(lockfile) {
  const installation = createInstallation('bypass');
  const brokenLockfile = 'lockfileVersion: [\n';
  const companion = join(external, 'vp-install-lock.yaml');
  writeFileSync(companion, brokenLockfile);
  // Only the exact value "1" enables the escape hatch.
  for (const value of [undefined, '', '0', 'true']) {
    await assert.rejects(
      runSetup(installation, { VP_SETUP_SKIP_INSTALL_LOCKFILE: value }),
      (error) => error.code === 1 && /ERR_PNPM_BROKEN_LOCKFILE/.test(error.message),
    );
  }
  console.log('A broken lockfile still fails unless VP_SETUP_SKIP_INSTALL_LOCKFILE is exactly 1.');

  const target = join(installation.home, version);
  // Keep the broken working copy, but remove both companions. Recovery would hit a 404.
  rmSync(companion);
  rmSync(join(target, 'bin/vp-install-lock.yaml'));
  const override = {
    VP_SETUP_SKIP_INSTALL_LOCKFILE: '1',
    npm_config_frozen_lockfile: 'true',
    NPM_CONFIG_FROZEN_LOCKFILE: 'true',
    npm_config_lockfile: 'true',
    NPM_CONFIG_LOCKFILE: 'true',
  };
  const before = requests.length;
  const result = await runSetup(installation, override);
  assert.match(
    result.stderr,
    /VP_SETUP_SKIP_INSTALL_LOCKFILE=1: ignoring the installation lockfile/,
  );
  const current = join(installation.home, 'current');
  const receipt = join(current, 'node_modules/.vp-install-complete');
  const marker = join(current, 'bin/.vp-setup-complete');
  assert.equal(readFileSync(join(current, 'pnpm-lock.yaml'), 'utf8'), brokenLockfile);
  assert.equal(existsSync(join(current, 'bin/vp-install-lock.yaml')), false);
  assert.equal(
    JSON.parse(readFileSync(join(current, 'node_modules/vite-plus/package.json'), 'utf8')).version,
    version,
  );
  assert.equal(readFileSync(receipt, 'utf8'), 'unlocked');
  assertDependencyDownloads(before);
  assert.equal(readFileSync(installation.userconfig, 'utf8'), installation.configuration);
  console.log(
    'The bypass warns, ignores broken or missing lockfiles, and installs the exact CLI version.',
  );
  console.log(
    'User registry and authentication still work; the completion receipt is explicitly unlocked.',
  );

  // Exercise the real upgrade path with an archive that advertises a missing companion.
  upgradeArchive = createArchive({
    'package/package.json': JSON.stringify({ version, vpInstallLockfile: true }),
    'package/vp': readFileSync(join(runnerHome, 'bin/vp')),
  });
  upgradeIntegrity = `sha512-${createHash('sha512').update(upgradeArchive).digest('base64')}`;
  const upgradeArgs = ['upgrade', version, '--force', '--registry', registry];
  const previousTarget = readlinkSync(current);
  await assert.rejects(
    runSetup(installation, {}, join(current, 'bin/vp'), upgradeArgs),
    (error) =>
      error.code === 1 && /Platform package is missing vp-install-lock.yaml/.test(error.message),
  );
  assert.equal(readlinkSync(current), previousTarget);
  const upgradeResult = await runSetup(
    installation,
    override,
    join(current, 'bin/vp'),
    upgradeArgs,
  );
  assert.match(
    upgradeResult.stderr,
    /VP_SETUP_SKIP_INSTALL_LOCKFILE=1: ignoring the installation lockfile/,
  );
  assert.notEqual(readlinkSync(current), previousTarget);
  assert.equal(existsSync(join(current, 'bin/vp-install-lock.yaml')), false);
  assert.equal(existsSync(marker), true);
  assert.equal(readFileSync(receipt, 'utf8'), 'unlocked');
  await runSetup(installation, {}, join(current, 'bin/vp'), ['--version']);
  console.log('Upgrade rejects a missing companion by default and succeeds with the bypass.');
  console.log('Upgrade handoff completes setup; later commands work without the override.');

  // A subsequent normal setup must not accept an unlocked receipt as frozen.
  writeFileSync(join(current, 'bin/vp-install-lock.yaml'), lockfile);
  rmSync(marker);
  await runSetup(installation, {}, join(current, 'bin/vp'));
  assert.equal(readFileSync(join(current, 'pnpm-lock.yaml'), 'utf8'), lockfile);
  assert.deepEqual(readFileSync(receipt), createHash('sha256').update(lockfile).digest());
  console.log('Normal setup restores the published lockfile and records a fresh frozen install.');

  const failed = createInstallation('failed-bypass');
  const failedTarget = join(failed.home, version);
  const failedReceipt = join(failedTarget, 'node_modules/.vp-install-complete');
  mkdirSync(join(failedTarget, 'node_modules'), { recursive: true });
  writeFileSync(failedReceipt, createHash('sha256').update(lockfile).digest());
  // A fresh offline store makes resolution fail without any external requests or retries.
  await assert.rejects(
    runSetup(failed, { ...override, npm_config_offline: 'true', NPM_CONFIG_OFFLINE: 'true' }),
    (error) => error.code === 1 && /ERR_PNPM_NO_OFFLINE_META/.test(error.message),
  );
  assert.equal(existsSync(failedReceipt), false);
  assert.equal(existsSync(join(failedTarget, 'bin/.vp-setup-complete')), false);
  console.log('A failed bypass clears any stale receipt and leaves setup incomplete.');
}

async function testInstallConfig(lockfile) {
  for (const conflict of [...conflicts, disabledLockfile]) {
    const installation = createInstallation(conflict.option, [conflict]);
    const { directory, home, userconfig, configuration, env } = installation;
    const control = join(directory, 'control');
    mkdirSync(control);
    writeFileSync(join(control, 'package.json'), JSON.stringify(installManifest(version)));
    writeFileSync(join(control, '.npmrc'), 'minimum-release-age=0\n');
    writeFileSync(join(control, 'pnpm-lock.yaml'), lockfile);
    await assert.rejects(
      runPnpm(control, ['install', '--ignore-workspace', '--frozen-lockfile'], env),
      (error) => {
        if (error.code !== 1) return false;
        const output = error.stdout + error.stderr;
        if (conflict.option === 'lockfile') {
          return /ERR_PNPM_NO_LOCKFILE/.test(output);
        }
        return (
          /ERR_PNPM_LOCKFILE_CONFIG_MISMATCH/.test(output) &&
          output.includes(`settings.${conflict.setting}`)
        );
      },
    );
    assert.equal(readFileSync(join(control, 'pnpm-lock.yaml'), 'utf8'), lockfile);
    console.log(`Without the installer override, real pnpm rejects ${conflict.option}.`);

    const before = requests.length;
    await runSetup(installation, conflictEnvironment([conflict]));
    const current = join(home, 'current');
    assert.equal(readFileSync(join(current, 'pnpm-lock.yaml'), 'utf8'), lockfile);
    assert.deepEqual(
      readFileSync(join(current, 'node_modules/.vp-install-complete')),
      createHash('sha256').update(lockfile).digest(),
    );
    assert.equal(existsSync(join(current, 'node_modules/vite-plus/package.json')), true);
    assertDependencyDownloads(before);
    assert.equal(readFileSync(userconfig, 'utf8'), configuration);
    console.log(`Frozen setup overrides ${conflict.option} in user and environment configuration.`);
  }
  console.log(
    'All installs fetch the CLI and its peer using the user registry and authentication.',
  );
  console.log('The published lockfile and user configuration remain unchanged.');
}

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
  if (bypass) {
    await testBypass(lockfile);
  } else {
    await testInstallConfig(lockfile);
  }
  assert.ok(requests.every(({ authorization }) => authorization === 'Bearer fixture-token'));
} finally {
  server.close();
  server.closeAllConnections();
  rmSync(root, { recursive: true, force: true });
}
