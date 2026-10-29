import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { once } from 'node:events';
import fs from 'node:fs';
import { createServer } from 'node:http';
import path from 'node:path';

import { createArchive } from './archive.mjs';

const root = path.resolve('homebrew-test');
const source =
  process.env.TEST_VP_BINARY ?? fs.realpathSync(path.join(process.env.VP_HOME, 'bin/vp'));
const node = fs.realpathSync(process.execPath);
const npmRoot = path.resolve(path.dirname(node), '../lib/node_modules/npm');
assert.ok(
  fs.existsSync(path.join(npmRoot, 'bin/npm-cli.js')),
  'fixture needs Node with bundled npm',
);
fs.mkdirSync(root, { recursive: true });
const originalEnv = Object.fromEntries(
  Object.entries(process.env).filter(([key]) => !/^(VP_|XDG_|npm_|NPM_|PNPM_)/.test(key)),
);
const token = 'synthetic-homebrew-token';
const pin = '10.34.7';
const lockfile = "lockfileVersion: '9.0'\n";
fs.writeFileSync(
  path.join(root, 'package.json'),
  JSON.stringify({ name: 'homebrew-fixture', packageManager: `pnpm@${pin}` }),
);

async function run(binary, args, env, cwd = root, status = 0) {
  const child = spawn(binary, args, { cwd, env, stdio: ['ignore', 'pipe', 'pipe'] });
  let stdout = '',
    stderr = '';
  child.stdout.setEncoding('utf8').on('data', (chunk) => {
    stdout += chunk;
  });
  child.stderr.setEncoding('utf8').on('data', (chunk) => {
    stderr += chunk;
  });
  const [code, signal] = await once(child, 'close');
  assert.equal(signal, null, stderr);
  assert.equal(code, status, stdout + stderr);
  assert.ok(!stdout.includes(token) && !stderr.includes(token), 'credentials leaked');
  return { stdout, stderr };
}

const pnpmCode = `
  const fs = require('node:fs');
  const assert = require('node:assert/strict');
  if (process.argv.includes('install')) {
    if (process.env.TEST_USERCONFIG) {
      assert.equal(process.env.npm_config_userconfig, process.env.TEST_USERCONFIG);
      assert.equal(process.env.NPM_CONFIG_USERCONFIG, process.env.TEST_USERCONFIG);
      assert.ok(fs.readFileSync(process.env.npm_config_userconfig, 'utf8').includes('registry=' + process.env.TEST_REGISTRY + '/registry/'));
    }
    assert.ok(process.argv.includes('--frozen-lockfile'));
    assert.equal(fs.readFileSync('pnpm-lock.yaml', 'utf8'), ${JSON.stringify(lockfile)});
    const version = JSON.parse(fs.readFileSync('package.json')).dependencies['vite-plus'];
    fetch(process.env.TEST_REGISTRY + '/installed').then(response => {
      if (!response.ok) { console.error(process.env.NPM_TOKEN); process.exit(17); }
      fs.mkdirSync('node_modules/vite-plus/dist', { recursive: true });
      fs.writeFileSync('node_modules/vite-plus/package.json', JSON.stringify({ name: 'vite-plus', version }));
      fs.writeFileSync('node_modules/vite-plus/dist/bin.js', "require('node:fs').writeFileSync(process.env.TEST_CLI_BINARY, process.env.VP_CLI_BIN); console.log('homebrew user CLI')");
    });
  } else { console.log('pinned pnpm tool'); }
`;
const archive = createArchive({
  'package/package.json': JSON.stringify({
    name: 'pnpm',
    version: pin,
    engines: { node: '>=18.12' },
    bin: { pnpm: 'bin/pnpm.cjs', pnpx: 'bin/pnpx.cjs' },
  }),
  'package/bin/pnpm.cjs': pnpmCode,
  'package/bin/pnpx.cjs': pnpmCode,
});
const integrity = 'sha512-' + createHash('sha512').update(archive).digest('base64');
// A small local Node distribution keeps fresh-root setup independent of the network.
const nodeArchives = new Map(
  ['darwin-x64', 'darwin-arm64', 'linux-x64', 'linux-arm64', 'linux-x64-musl', 'linux-arm64-musl'].map(
    (platform) => {
      const directory = `node-v99.0.0-${platform}`;
      return [
        `${directory}.tar.gz`,
        createArchive({ [`${directory}/bin/node`]: '#!/bin/sh\nexec "$TEST_SYSTEM_NODE" "$@"\n' }),
      ];
    },
  ),
);
let downloads = 0,
  installs = 0,
  registryRequests = 0,
  rejectInstall = false,
  corrupt = false;
let authorization = `Bearer ${token}`;
const server = createServer((request, response) => {
  if (request.url?.startsWith('/registry/')) registryRequests++;
  if (request.url === '/index.json')
    return response.end(JSON.stringify([{ version: 'v99.0.0', lts: 'Fixture' }]));
  if (request.url === '/v99.0.0/SHASUMS256.txt.asc') return response.writeHead(404).end();
  if (request.url === '/v99.0.0/SHASUMS256.txt') {
    return response.end(
      [...nodeArchives]
        .map(([name, data]) => `${createHash('sha256').update(data).digest('hex')}  ${name}\n`)
        .join(''),
    );
  }
  const nodeArchive = nodeArchives.get(request.url?.replace('/v99.0.0/', ''));
  if (nodeArchive) return response.end(nodeArchive);
  if (request.url === '/installed') {
    installs++;
    return response.writeHead(rejectInstall ? 500 : 200).end();
  }
  if (request.headers.authorization !== authorization)
    return response.writeHead(401).end('Unauthorized');
  if (request.url === `/registry/pnpm/${pin}`) {
    return response.end(
      JSON.stringify({
        name: 'pnpm',
        version: pin,
        dist: { tarball: `${url}/registry/custom-pnpm.tgz`, integrity },
      }),
    );
  }
  if (request.url === '/registry/custom-pnpm.tgz') {
    downloads++;
    return response.end(corrupt ? Buffer.concat([archive, Buffer.from('corrupt')]) : archive);
  }
  response.writeHead(404).end();
});
server.listen(0, '127.0.0.1');
await once(server, 'listening');
const url = `http://127.0.0.1:${server.address().port}`;
const publicVp = path.join(root, 'brew/bin/vp');

function keg(version) {
  const prefix = path.join(root, 'brew/Cellar/vp', version);
  fs.mkdirSync(path.join(prefix, 'bin'), { recursive: true });
  fs.mkdirSync(path.join(prefix, 'libexec'));
  fs.copyFileSync(source, path.join(prefix, 'libexec/vp'));
  fs.writeFileSync(path.join(prefix, 'libexec/vp-install-lock.yaml'), lockfile);
  fs.chmodSync(path.join(prefix, 'libexec/vp'), 0o555);
  fs.symlinkSync('../libexec/vp', path.join(prefix, 'bin/vp'));
  fs.writeFileSync(
    path.join(prefix, 'INSTALL_RECEIPT.json'),
    JSON.stringify({
      homebrew_version: '7.0.2',
      source: { tap: 'voidzero-dev/vite-plus', path: '/tap/HomebrewFormula/vp.rb' },
    }),
  );
  fs.chmodSync(path.join(prefix, 'bin'), 0o555);
  fs.chmodSync(path.join(prefix, 'libexec'), 0o555);
  fs.chmodSync(prefix, 0o555);
  return prefix;
}

function environment(name, auth = true, seedRuntime = true) {
  const home = path.join(root, name);
  const vpHome = path.join(home, '.vite-plus');
  fs.mkdirSync(home, { recursive: true });
  if (seedRuntime) {
    const nodeRoot = path.join(vpHome, 'js_runtime/node/99.0.0');
    fs.mkdirSync(path.join(nodeRoot, 'bin'), { recursive: true });
    fs.mkdirSync(path.join(nodeRoot, 'lib/node_modules'), { recursive: true });
    fs.symlinkSync(node, path.join(nodeRoot, 'bin/node'));
    fs.symlinkSync(npmRoot, path.join(nodeRoot, 'lib/node_modules/npm'));
    // These cases model an existing installation with a provisioned runtime.
    fs.writeFileSync(path.join(vpHome, '.vp-owned'), 'vite-plus-owned-directory\n');
  }
  fs.writeFileSync(
    path.join(home, '.npmrc'),
    `registry=${url}/registry/\n${auth ? `//127.0.0.1:${server.address().port}/registry/:_authToken=\${NPM_TOKEN}\n` : ''}`,
  );
  return {
    ...originalEnv,
    HOME: home,
    VP_HOME: vpHome,
    VP_SELF_SETUP_NO_MODIFY_PATH: '1',
    VP_NODE_DIST_MIRROR: url,
    VP_NODE_MANAGER: 'yes',
    VP_PM_MANAGER: 'no',
    VP_PNPM_MANAGER: 'yes',
    NPM_TOKEN: token,
    TEST_REGISTRY: url,
    TEST_SYSTEM_NODE: node,
    TEST_CLI_BINARY: path.join(home, 'vp-path'),
    npm_config_fetch_retries: '0',
    npm_config_cache: path.join(home, 'npm-cache'),
    PATH: [
      path.join(vpHome, 'bin'),
      path.join(root, 'brew/bin'),
      path.dirname(node),
      '/usr/bin',
      '/bin',
      path.join(vpHome, 'fallback-bin'),
    ].join(path.delimiter),
  };
}

function userPackage(env) {
  const versions = path.join(env.VP_HOME, 'cli-packages');
  const version = fs.readdirSync(versions)[0];
  return path.join(versions, version, fs.readdirSync(path.join(versions, version))[0]);
}

function unlock(prefix) {
  fs.chmodSync(prefix, 0o755);
  fs.chmodSync(path.join(prefix, 'bin'), 0o755);
  fs.chmodSync(path.join(prefix, 'libexec'), 0o755);
}

try {
  const old = keg('old');
  fs.mkdirSync(path.dirname(publicVp), { recursive: true });
  fs.symlinkSync(path.join(old, 'bin/vp'), publicVp);
  const env = environment('user');
  const userNpmrc = fs.readFileSync(path.join(env.HOME, '.npmrc'), 'utf8');
  rejectInstall = true;
  await run(publicVp, ['--help'], env, root, 1);
  const settingsFile = path.join(env.VP_HOME, 'config.json');
  const preferences = fs.readFileSync(settingsFile, 'utf8');
  assert.ok(!fs.existsSync(path.join(userPackage(env), 'node_modules/.vp-install-complete')));
  const failureLog = fs.readFileSync(path.join(userPackage(env), '../upgrade.log'), 'utf8');
  assert.ok(failureLog.includes('exit code 17') && !failureLog.includes(token));
  for (const name of ['VP_NODE_MANAGER', 'VP_PM_MANAGER', 'VP_PNPM_MANAGER']) delete env[name];
  rejectInstall = false;
  const retry = await Promise.all([run(publicVp, ['--help'], env), run(publicVp, ['--help'], env)]);
  assert.equal(installs, 2, 'retry must install once across concurrent launches');
  assert.equal(downloads, 1, 'the retry must reuse bootstrapped pnpm');
  assert.ok(retry.every((result) => !result.stderr.includes('manage Node')));
  assert.equal(fs.readFileSync(settingsFile, 'utf8'), preferences);
  assert.ok(!fs.existsSync(path.join(old, 'bin/.vp-setup-complete')));
  assert.equal(
    fs.readFileSync(path.join(userPackage(env), 'bin/vp-install-lock.yaml'), 'utf8'),
    lockfile,
  );
  assert.equal(
    fs
      .readFileSync(path.join(userPackage(env), 'node_modules/.vp-install-complete'))
      .toString('hex'),
    createHash('sha256').update(lockfile).digest('hex'),
  );
  console.log('First use copies the release lockfile and records a successful frozen install.');
  assert.ok(!fs.existsSync(path.join(env.VP_HOME, 'current')));
  assert.equal(fs.readlinkSync(path.join(env.VP_HOME, 'bin/vp')), publicVp);
  assert.equal(fs.readlinkSync(path.join(env.VP_HOME, 'fallback-bin/npm')), publicVp);
  console.log(
    'Authenticated first use retries once, preserves preferences, and serializes concurrent setup.',
  );

  const cli = await run(path.join(env.VP_HOME, 'bin/vp'), ['sync-versions', '--json'], env);
  assert.equal(cli.stdout.trim(), 'homebrew user CLI');
  const savedBinary = fs.readFileSync(env.TEST_CLI_BINARY, 'utf8');
  assert.equal(savedBinary, publicVp);
  const replacement = keg('new');
  fs.unlinkSync(publicVp);
  fs.symlinkSync(path.join(replacement, 'bin/vp'), publicVp);
  unlock(old);
  fs.rmSync(old, { recursive: true });
  await run(savedBinary, ['--help'], env);
  await run(path.join(env.VP_HOME, 'bin/vp'), ['--help'], env);
  await run(path.join(env.VP_HOME, 'bin/node'), ['--version'], env);
  for (const shim of ['pn', 'pnx']) {
    const output = await run(path.join(env.VP_HOME, 'bin', shim), ['--version'], env);
    assert.equal(output.stdout.trim(), 'pinned pnpm tool');
  }
  await run(path.join(env.VP_HOME, 'fallback-bin/npm'), ['--version'], env);
  assert.equal(fs.readFileSync(settingsFile, 'utf8'), preferences);
  assert.equal(installs, 2);
  console.log('CLI, Node, npm, pn, and pnx shims survive Cellar replacement and removal.');

  fs.writeFileSync(path.join(userPackage(env), 'pnpm-lock.yaml'), 'interrupted resolution');
  rejectInstall = true;
  await run(publicVp, ['--help'], env, root, 1);
  assert.ok(!fs.existsSync(path.join(userPackage(env), 'node_modules/.vp-install-complete')));
  rejectInstall = false;
  await run(publicVp, ['--help'], env);
  assert.equal(installs, 4);
  assert.equal(fs.readFileSync(settingsFile, 'utf8'), preferences);
  console.log('A failed frozen refresh invalidates completion and retries with saved preferences.');

  const sharedReceipt = path.join(userPackage(env), 'node_modules/.vp-install-complete');
  fs.unlinkSync(sharedReceipt);
  const incomplete = await run(
    publicVp,
    ['--help'],
    { ...env, VP_SKIP_DEPS_INSTALL: '1' },
    root,
    1,
  );
  assert.ok(incomplete.stderr.includes('no matching completion receipt'));
  assert.equal(installs, 4, 'a dependency bypass must fail without entering a setup loop');
  await run(publicVp, ['--help'], env);
  assert.equal(installs, 5, 'missing shared receipt must trigger dependency repair');
  fs.writeFileSync(sharedReceipt, 'corrupt');
  await run(publicVp, ['--help'], env);
  assert.equal(installs, 6, 'invalid shared receipt must trigger dependency repair');
  // The binary and version stay unchanged, but the companion now identifies a new payload.
  const publishedLock = path.join(replacement, 'libexec/vp-install-lock.yaml');
  fs.writeFileSync(publishedLock, lockfile + '# replacement companion\n');
  const copiedLock = path.join(userPackage(env), 'bin/vp-install-lock.yaml');
  rejectInstall = true;
  await run(publicVp, ['--help'], env, root, 1);
  assert.equal(fs.readFileSync(copiedLock, 'utf8'), lockfile + '# replacement companion\n');
  assert.ok(!fs.existsSync(sharedReceipt));
  fs.writeFileSync(publishedLock, lockfile);
  rejectInstall = false;
  await run(publicVp, ['--help'], env);
  assert.equal(installs, 7, 'restoring the companion must repair its dependency installation');
  const repaired = fs.readFileSync(sharedReceipt);
  assert.equal(repaired.toString('hex'), createHash('sha256').update(lockfile).digest('hex'));
  await run(publicVp, ['--help'], env);
  assert.equal(installs, 7, 'completed repair must be reused');
  assert.equal(fs.readFileSync(settingsFile, 'utf8'), preferences);
  console.log('Missing receipts, corrupt receipts, and changed companions trigger dependency repair.');

  const upgrade = await run(publicVp, ['upgrade'], env, root, 1);
  assert.ok(upgrade.stderr.includes('brew upgrade voidzero-dev/vite-plus/vp'));
  const outdated = await run(publicVp, ['upgrade', '--check'], env);
  assert.ok(
    (outdated.stdout + outdated.stderr).includes('brew outdated voidzero-dev/vite-plus/vp'),
  );
  const doctor = await run(publicVp, ['env', 'doctor', 'node'], env);
  assert.ok(doctor.stdout.includes('CLI dependencies'));
  const secondUser = environment('second-user');
  // Model migration from a script install with a managed global package.
  const previous = path.join(secondUser.VP_HOME, 'current/bin/vp');
  fs.mkdirSync(path.dirname(previous), { recursive: true });
  fs.copyFileSync(source, previous);
  fs.mkdirSync(path.join(secondUser.VP_HOME, 'bins'));
  const metadata = path.join(secondUser.VP_HOME, 'bins/global-tool.json');
  fs.writeFileSync(
    metadata,
    JSON.stringify({
      name: 'global-tool',
      package: 'global-tool',
      version: '1.0.0',
      nodeVersion: '99.0.0',
      source: 'vp',
    }),
  );
  fs.mkdirSync(path.join(secondUser.VP_HOME, 'bin'));
  const globalShim = path.join(secondUser.VP_HOME, 'bin/global-tool');
  fs.symlinkSync(previous, globalShim);
  await run(publicVp, ['--help'], secondUser);
  assert.equal(installs, 8);
  assert.notEqual(userPackage(env), userPackage(secondUser));
  assert.equal(fs.readlinkSync(globalShim), publicVp);
  assert.ok(fs.existsSync(metadata) && fs.existsSync(previous));
  const removed = await run(publicVp, ['implode', '--yes'], env);
  assert.ok((removed.stdout + removed.stderr).includes('brew uninstall voidzero-dev/vite-plus/vp'));
  assert.ok(!fs.existsSync(env.VP_HOME));
  assert.equal(fs.readFileSync(path.join(env.HOME, '.npmrc'), 'utf8'), userNpmrc);
  assert.ok(fs.existsSync(publicVp));
  assert.ok(fs.existsSync(userPackage(secondUser)));
  console.log(
    'Ownership commands use the tap name; migration preserves global packages and refreshes their shims.',
  );
  for (const split of [false, true]) {
    const fresh = environment(`fresh-${split ? 'split' : 'single'}`, true, false);
    const data = split ? path.join(fresh.HOME, 'data') : fresh.VP_HOME;
    const cache = split ? path.join(fresh.HOME, 'cache') : path.join(data, 'cache');
    const bin = split ? path.join(fresh.HOME, 'bin') : path.join(data, 'bin');
    const config = split ? path.join(fresh.HOME, 'config/vite-plus') : data;
    const state = split ? path.join(fresh.HOME, 'state/vite-plus') : data;
    if (split) {
      delete fresh.VP_HOME;
      Object.assign(fresh, {
        VP_BIN_DIR: bin,
        VP_DATA_DIR: data,
        VP_CACHE_DIR: cache,
        XDG_CONFIG_HOME: path.dirname(config),
        XDG_STATE_HOME: path.dirname(state),
      });
      fresh.PATH = [bin, path.dirname(publicVp), path.dirname(node), '/usr/bin', '/bin'].join(
        path.delimiter,
      );
    }
    const roots = [...new Set([data, cache, config, state])];
    const npmrc = fs.readFileSync(path.join(fresh.HOME, '.npmrc'), 'utf8');
    for (let attempt = 0; attempt < 2; attempt++) {
      for (const directory of roots) assert.ok(!fs.existsSync(directory), directory);
      const before = installs;
      await Promise.all([run(publicVp, ['--help'], fresh), run(publicVp, ['--help'], fresh)]);
      assert.equal(installs, before + 1, 'fresh concurrent setup must install once');
      for (const directory of roots) {
        assert.equal(
          fs.readFileSync(path.join(directory, '.vp-owned'), 'utf8'),
          'vite-plus-owned-directory\n',
        );
      }
      assert.equal(fs.readlinkSync(path.join(bin, 'vp')), publicVp);
      assert.equal(fs.readlinkSync(path.join(data, 'fallback-bin/npm')), publicVp);
      await run(path.join(bin, 'node'), ['--version'], fresh);
      await run(publicVp, ['implode', '--yes'], fresh);
      for (const directory of roots) assert.ok(!fs.existsSync(directory), directory);
      assert.ok(!fs.existsSync(path.join(bin, 'vp')));
      assert.ok(fs.existsSync(publicVp));
      assert.equal(fs.readFileSync(path.join(fresh.HOME, '.npmrc'), 'utf8'), npmrc);
    }
  }
  console.log('User installations and cleanup stay independent.');

  for (const [name, registry] of [
    ['direct', `${url}/registry/`],
    ['interpolated', '${TEST_REGISTRY}/registry/'],
  ]) {
    const override = environment(`${name}-registry-override`);
    const config = path.join(override.HOME, '.npmrc');
    fs.writeFileSync(
      config,
      fs
        .readFileSync(config, 'utf8')
        .replace(`${url}/registry/`, 'http://127.0.0.1:1/unavailable/'),
    );
    override.NPM_CONFIG_REGISTRY = registry;
    await run(publicVp, ['--help'], override);
  }

  for (const [name, selector] of [
    ['relative', null],
    ['tilde', '~/enterprise.npmrc'],
    ['environment', '${HOME}/enterprise.npmrc'],
  ]) {
    const userconfig = environment(`${name}-userconfig`);
    const userconfigFile = path.join(userconfig.HOME, 'enterprise.npmrc');
    fs.renameSync(path.join(userconfig.HOME, '.npmrc'), userconfigFile);
    userconfig.NPM_CONFIG_USERCONFIG = selector ?? path.relative(root, userconfigFile);
    userconfig.TEST_USERCONFIG = userconfigFile;
    await run(publicVp, ['--help'], userconfig);
  }
  console.log(
    'Relative, tilde, and environment-variable selectors reach pnpm as the same absolute user config.',
  );

  const basic = environment('basic-auth');
  const encoded = Buffer.from('fixture:synthetic-password').toString('base64');
  fs.writeFileSync(
    path.join(basic.HOME, '.npmrc'),
    `registry=${url}/registry/\n//127.0.0.1:${server.address().port}/registry/:_auth=${encoded}\n`,
  );
  authorization = `Basic ${encoded}`;
  await run(publicVp, ['--help'], basic);
  authorization = `Bearer ${token}`;
  console.log('Registry overrides and basic authentication work with the shared downloader.');

  const unauthenticated = environment('unauthenticated', false);
  await run(publicVp, ['--help'], unauthenticated, root, 1);
  const requestsBeforeRecovery = registryRequests;
  const diagnostic = await run(
    publicVp,
    ['-C', root, 'env', 'doctor', 'node'],
    unauthenticated,
    root,
    1,
  );
  assert.ok(diagnostic.stdout.includes('CLI dependencies'));
  const checkAfterFailure = await run(publicVp, ['upgrade', '--check'], unauthenticated);
  assert.ok(
    (checkAfterFailure.stdout + checkAfterFailure.stderr).includes(
      'brew outdated voidzero-dev/vite-plus/vp',
    ),
  );
  const upgradeAfterFailure = await run(publicVp, ['upgrade'], unauthenticated, root, 1);
  assert.ok(upgradeAfterFailure.stderr.includes('brew upgrade voidzero-dev/vite-plus/vp'));
  const cleanupAfterFailure = await run(publicVp, ['implode', '--yes'], unauthenticated);
  assert.ok(
    (cleanupAfterFailure.stdout + cleanupAfterFailure.stderr).includes(
      'brew uninstall voidzero-dev/vite-plus/vp',
    ),
  );
  assert.ok(!fs.existsSync(unauthenticated.VP_HOME));
  assert.ok(fs.existsSync(publicVp));
  assert.equal(registryRequests, requestsBeforeRecovery, 'recovery must not retry npm setup');
  console.log(
    'After authentication failure, diagnostics, upgrade guidance, and cleanup remain available.',
  );

  for (const installerEnv of [
    { VP_SELF_SETUP_SHELL: 'sh' },
    { VP_SELF_SETUP_REPLACE_EXISTING: '1' },
  ]) {
    const explicit = environment(`explicit-${Object.keys(installerEnv)[0]}`, false);
    const requestsBefore = registryRequests;
    await run(publicVp, ['upgrade', '--check'], { ...explicit, ...installerEnv }, root, 1);
    assert.ok(registryRequests > requestsBefore, 'explicit setup must still install dependencies');
  }
  const alias = path.join(root, 'brew/bin/vpr');
  fs.symlinkSync(publicVp, alias);
  const requestsBeforeAlias = registryRequests;
  await run(alias, ['upgrade', '--check'], environment('alias', false), root, 1);
  assert.ok(registryRequests > requestsBeforeAlias, 'shim arguments must not bypass setup');

  corrupt = true;
  await run(publicVp, ['--help'], environment('corrupt'), root, 1);
  console.log('Missing authentication and corrupted pnpm tarballs stop setup.');
  unlock(replacement);
} finally {
  server.close();
  server.closeAllConnections();
}
