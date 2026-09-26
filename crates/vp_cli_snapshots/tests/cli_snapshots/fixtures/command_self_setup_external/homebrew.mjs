import assert from 'node:assert/strict';
import { spawn, spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { once } from 'node:events';
import fs from 'node:fs';
import { createServer } from 'node:http';
import path from 'node:path';

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
const pin = '10.33.0';
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

const packageRoot = path.join(root, 'fake-pnpm');
fs.mkdirSync(path.join(packageRoot, 'bin'), { recursive: true });
fs.writeFileSync(
  path.join(packageRoot, 'package.json'),
  JSON.stringify({
    name: 'pnpm',
    version: pin,
    bin: { pnpm: 'bin/pnpm.cjs', pnpx: 'bin/pnpx.cjs' },
  }),
);
const pnpmCode = `
  const fs = require('node:fs');
  if (process.argv.includes('install')) {
    const version = JSON.parse(fs.readFileSync('package.json')).dependencies['vite-plus'];
    fetch(process.env.TEST_REGISTRY + '/installed').then(response => {
      if (!response.ok) { console.error(process.env.NPM_TOKEN); process.exit(17); }
      fs.mkdirSync('node_modules/vite-plus/dist', { recursive: true });
      fs.writeFileSync('node_modules/vite-plus/package.json', JSON.stringify({ name: 'vite-plus', version }));
      fs.writeFileSync('node_modules/vite-plus/dist/bin.js', "require('node:fs').writeFileSync(process.env.TEST_CLI_BINARY, process.env.VP_CLI_BIN); console.log('homebrew user CLI')");
    });
  } else { console.log('pinned pnpm tool'); }
`;
for (const name of ['pnpm', 'pnpx'])
  fs.writeFileSync(path.join(packageRoot, 'bin', `${name}.cjs`), pnpmCode);
const packEnv = { ...originalEnv, HOME: root, npm_config_cache: path.join(root, 'pack-cache') };
const packed = spawnSync(
  node,
  [
    path.join(npmRoot, 'bin/npm-cli.js'),
    'pack',
    packageRoot,
    '--ignore-scripts',
    '--json',
    '--workspaces=false',
  ],
  { cwd: root, env: packEnv, encoding: 'utf8' },
);
assert.equal(packed.status, 0, packed.stderr);
const archive = fs.readFileSync(path.join(root, JSON.parse(packed.stdout)[0].filename));
const integrity = 'sha512-' + createHash('sha512').update(archive).digest('base64');
let downloads = 0,
  installs = 0,
  rejectInstall = false,
  corrupt = false;
let authorization = `Bearer ${token}`;
const server = createServer((request, response) => {
  if (request.url === '/index.json')
    return response.end(JSON.stringify([{ version: 'v99.0.0', lts: 'Fixture' }]));
  if (request.url === '/installed') {
    installs++;
    return response.writeHead(rejectInstall ? 500 : 200).end();
  }
  if (request.headers.authorization !== authorization)
    return response.writeHead(401).end('Unauthorized');
  if (request.url === '/registry/pnpm') {
    return response.end(
      JSON.stringify({
        name: 'pnpm',
        'dist-tags': { latest: pin },
        versions: {
          [pin]: {
            name: 'pnpm',
            version: pin,
            dist: { tarball: `${url}/registry/custom-pnpm.tgz`, integrity },
          },
        },
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
  fs.copyFileSync(source, path.join(prefix, 'bin/vp'));
  fs.chmodSync(path.join(prefix, 'bin/vp'), 0o555);
  fs.writeFileSync(
    path.join(prefix, 'INSTALL_RECEIPT.json'),
    JSON.stringify({
      homebrew_version: '7.0.2',
      source: { tap: 'voidzero-dev/vite-plus', path: '/tap/HomebrewFormula/vp.rb' },
    }),
  );
  fs.chmodSync(path.join(prefix, 'bin'), 0o555);
  fs.chmodSync(prefix, 0o555);
  return prefix;
}

function environment(name, auth = true) {
  const home = path.join(root, name);
  const nodeRoot = path.join(home, 'js_runtime/node/99.0.0');
  fs.mkdirSync(path.join(nodeRoot, 'bin'), { recursive: true });
  fs.mkdirSync(path.join(nodeRoot, 'lib/node_modules'), { recursive: true });
  fs.symlinkSync(node, path.join(nodeRoot, 'bin/node'));
  fs.symlinkSync(npmRoot, path.join(nodeRoot, 'lib/node_modules/npm'));
  fs.writeFileSync(
    path.join(home, '.npmrc'),
    `registry=${url}/registry/\n${auth ? `//127.0.0.1:${server.address().port}/registry/:_authToken=\${NPM_TOKEN}\n` : ''}`,
  );
  return {
    ...originalEnv,
    HOME: home,
    VP_HOME: home,
    VP_SELF_SETUP_NO_MODIFY_PATH: '1',
    VP_NODE_DIST_MIRROR: url,
    VP_NODE_MANAGER: 'yes',
    VP_PM_MANAGER: 'no',
    VP_PNPM_MANAGER: 'yes',
    NPM_TOKEN: token,
    TEST_REGISTRY: url,
    TEST_CLI_BINARY: path.join(home, 'vp-path'),
    npm_config_fetch_retries: '0',
    npm_config_cache: path.join(home, 'npm-cache'),
    PATH: [
      path.join(home, 'bin'),
      path.join(root, 'brew/bin'),
      path.dirname(node),
      '/usr/bin',
      '/bin',
      path.join(home, 'fallback-bin'),
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
}

try {
  const old = keg('old');
  fs.mkdirSync(path.dirname(publicVp), { recursive: true });
  fs.symlinkSync(path.join(old, 'bin/vp'), publicVp);
  const env = environment('user');
  rejectInstall = true;
  await run(publicVp, ['--help'], env, root, 1);
  const settingsFile = path.join(env.VP_HOME, 'config.json');
  const preferences = fs.readFileSync(settingsFile, 'utf8');
  assert.ok(!fs.existsSync(path.join(userPackage(env), '.vp-deps-complete')));
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
  assert.equal(installs, 3);
  assert.notEqual(userPackage(env), userPackage(secondUser));
  assert.equal(fs.readlinkSync(globalShim), publicVp);
  assert.ok(fs.existsSync(metadata) && fs.existsSync(previous));
  const removed = await run(publicVp, ['implode', '--yes'], env);
  assert.ok((removed.stdout + removed.stderr).includes('brew uninstall voidzero-dev/vite-plus/vp'));
  assert.ok(!fs.existsSync(env.VP_HOME));
  assert.ok(fs.existsSync(publicVp));
  assert.ok(fs.existsSync(userPackage(secondUser)));
  console.log(
    'Ownership commands use the tap name; migration preserves global packages and refreshes their shims.',
  );
  console.log('User installations and cleanup stay independent.');

  const override = environment('registry-override');
  const config = path.join(override.HOME, '.npmrc');
  fs.writeFileSync(
    config,
    fs.readFileSync(config, 'utf8').replace(`${url}/registry/`, 'http://127.0.0.1:1/unavailable/'),
  );
  override.NPM_CONFIG_REGISTRY = `${url}/registry/`;
  await run(publicVp, ['--help'], override);

  const userconfig = environment('relative-userconfig');
  const userconfigFile = path.join(userconfig.HOME, 'enterprise.npmrc');
  fs.renameSync(path.join(userconfig.HOME, '.npmrc'), userconfigFile);
  userconfig.NPM_CONFIG_USERCONFIG = path.relative(root, userconfigFile);
  await run(publicVp, ['--help'], userconfig);

  const basic = environment('basic-auth');
  const encoded = Buffer.from('fixture:synthetic-password').toString('base64');
  fs.writeFileSync(
    path.join(basic.HOME, '.npmrc'),
    `registry=${url}/registry/\n//127.0.0.1:${server.address().port}/registry/:_auth=${encoded}\n`,
  );
  authorization = `Basic ${encoded}`;
  await run(publicVp, ['--help'], basic);
  authorization = `Bearer ${token}`;
  console.log(
    'Registry overrides, relative user configuration, and basic authentication work with bundled npm.',
  );

  await run(publicVp, ['--help'], environment('unauthenticated', false), root, 1);
  corrupt = true;
  await run(publicVp, ['--help'], environment('corrupt'), root, 1);
  console.log('Missing authentication and corrupted pnpm tarballs stop setup.');
  unlock(replacement);
} finally {
  server.close();
  server.closeAllConnections();
}
