import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { randomBytes } from 'node:crypto';
import { once } from 'node:events';
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs';
import { createServer } from 'node:http';
import { join, resolve } from 'node:path';

import { createArchive } from './archive.mjs';

const interactive = process.argv.includes('interactive');
const failure = process.argv.includes('failure');
const resolutionRetry = process.argv.includes('resolution-retry');
const dependencyRetry = process.argv.includes('dependency-retry');
const upgradeHandoff = process.argv.includes('upgrade-handoff');
const captureStderr = resolutionRetry || dependencyRetry || upgradeHandoff;
if (interactive) process.stdin.setRawMode(true);
const home = resolve('home');
const user = resolve('user');
const external = resolve('external');
const resolutionDir = resolve('resolution');
const resolutionFile = join(resolutionDir, 'node-version');
const nodeBin = join(home, 'js_runtime/node/99.0.0/bin');
for (const directory of [nodeBin, user, external]) mkdirSync(directory, { recursive: true });
// Use the runner's real Node for the mock pnpm program, without a network download.
symlinkSync(process.execPath, join(nodeBin, 'node'));
copyFileSync(join(process.env.VP_HOME, 'bin/vp'), join(external, 'vp'));
const lockfile = "lockfileVersion: '9.0'\n";
writeFileSync(join(external, 'vp-install-lock.yaml'), lockfile);

const pnpmArchive = createArchive({
  'package/package.json': JSON.stringify({
    name: 'pnpm',
    version: '10.34.7',
    bin: { pnpm: 'bin/pnpm.cjs' },
  }),
  'package/bin/pnpm.cjs': `
    const fs = require('node:fs');
    const assert = require('node:assert/strict');
    assert.ok(process.argv.includes('--frozen-lockfile'));
    assert.equal(fs.readFileSync('pnpm-lock.yaml', 'utf8'), ${JSON.stringify(lockfile)});
    console.log('pnpm captured stdout');
    console.error('pnpm captured stderr');
    fetch(process.env.npm_config_registry + '/install').then(response => {
      fs.mkdirSync('node_modules/vite-plus', { recursive: true });
      fs.writeFileSync('node_modules/vite-plus/package.json', '{"name":"vite-plus"}');
      if (!response.ok) process.exit(17);
    });
  `,
});

// The request is made after the CLI draws each phase. Hold its response until
// the runner captures the milestone, without sleeps or polling terminal text.
const phases = [];
/** @returns {Promise<void>} */
async function checkpoint(phase) {
  phases.push(phase);
  if (!interactive) return;
  const nextInput = once(process.stdin, 'data');
  process.stdin.resume();
  const id = randomBytes(16).toString('hex');
  const name = Buffer.from(`install:${phase}:ready`).toString('base64url');
  process.stderr.write(`\x1b]2;vt-pty-terminal-test:${id}:${name}\x1b\\`);
  await nextInput;
  process.stdin.pause();
}

/** @returns {Promise<void>} */
async function handleRequest(request, response) {
  if (request.url === '/index.json') {
    await checkpoint('prepare');
    response.end(JSON.stringify([{ version: 'v99.0.0', lts: 'Fixture' }]));
  } else if (/^\/pnpm\/-\/pnpm-[\d.]+\.tgz$/.test(request.url)) {
    await checkpoint('download');
    response.setHeader('Content-Length', pnpmArchive.length);
    response.end(pnpmArchive);
  } else if (request.url === '/install') {
    await checkpoint('dependencies');
    const fail =
      failure ||
      (dependencyRetry && phases.filter((phase) => phase === 'dependencies').length === 1);
    response.writeHead(fail ? 500 : 200).end();
  } else {
    response.writeHead(404).end();
  }
}

const server = createServer(handleRequest);
server.listen(0, '127.0.0.1');
await once(server, 'listening');
const mirror = `http://127.0.0.1:${server.address().port}`;

async function runInstaller(binary = join(external, 'vp'), args = []) {
  const child = spawn(binary, args, {
    stdio: ['ignore', 'pipe', captureStderr ? 'pipe' : 'inherit'],
    env: {
      ...process.env,
      HOME: user,
      PATH: nodeBin,
      VP_HOME: home,
      VP_NODE_DIST_MIRROR: mirror,
      npm_config_registry: mirror,
      NPM_CONFIG_REGISTRY: mirror,
      VP_SELF_SETUP_SHELL: args.length === 0 ? 'sh' : undefined,
      VP_SELF_SETUP_NO_MODIFY_PATH: '1',
      VP_NODE_MANAGER: 'no',
      VP_SHELL: 'zsh',
      VP_SKIP_DEPS_INSTALL: '',
      VP_SETUP_NODE_VERSION: resolutionRetry ? '20.18.0' : '',
      VP_SETUP_NODE_VERSION_FILE: resolutionRetry ? resolutionFile : '',
    },
  });
  let stdout = '';
  let stderr = '';
  child.stdout.setEncoding('utf8').on('data', (chunk) => {
    stdout += chunk;
  });
  child.stderr?.setEncoding('utf8').on('data', (chunk) => {
    stderr += chunk;
  });
  const [code, signal] = await once(child, 'exit');
  assert.equal(signal, null);
  return { code, stdout, stderr };
}

try {
  console.log('Before installation: preserve this output.');
  let result = await runInstaller();
  if (resolutionRetry || dependencyRetry) {
    assert.equal(result.code, 1, result.stderr);
    assert.equal(result.stdout, '');
    const installed = readdirSync(home)
      .map((name) => join(home, name))
      .filter((directory) => existsSync(join(directory, 'node_modules/vite-plus/package.json')));
    assert.equal(installed.length, 1);
    const marker = join(installed[0], 'bin/.vp-setup-complete');
    const receipt = join(installed[0], 'node_modules/.vp-install-complete');
    assert.equal(existsSync(marker), false);
    assert.equal(existsSync(receipt), resolutionRetry);
    assert.equal(existsSync(resolutionFile), false);
    assert.deepEqual(phases, ['download', 'prepare', 'dependencies']);
    console.log(
      resolutionRetry
        ? 'A failed resolution-file write leaves dependencies installed and setup incomplete.'
        : 'A failed dependency install leaves package.json present, without a completion receipt.',
    );

    if (resolutionRetry) mkdirSync(resolutionDir);
    result = await runInstaller();
    assert.equal(result.code, 0, result.stderr);
    if (resolutionRetry) assert.equal(readFileSync(resolutionFile, 'utf8'), '20.18.0');
    assert.equal(existsSync(marker), true);
    assert.equal(existsSync(receipt), true);
    console.log(
      resolutionRetry
        ? 'Retry reuses the completed frozen install and returns the requested Node version.'
        : 'Retry completes the frozen dependency install before marking setup complete.',
    );
  }
  if (upgradeHandoff) {
    assert.equal(result.code, 0, result.stderr);
    const current = join(home, 'current');
    const marker = join(current, 'bin/.vp-setup-complete');
    assert.equal(existsSync(join(current, 'node_modules/.vp-install-complete')), true);
    // Upgrade completes the shared dependency installer, then clears only the setup marker.
    rmSync(marker);
    result = await runInstaller(join(current, 'bin/vp'), ['--help']);
    assert.equal(result.code, 0, result.stderr);
    assert.equal(existsSync(marker), true);
    console.log(
      'Upgrade handoff completes self-setup without another runtime lookup or dependency install.',
    );
  }
  const { code, stdout, stderr } = result;
  assert.equal(code, failure ? 1 : 0, stderr);
  assert.deepEqual(
    phases,
    dependencyRetry
      ? ['download', 'prepare', 'dependencies', 'dependencies']
      : ['download', 'prepare', 'dependencies'],
  );
  if (failure) {
    assert.equal(stdout, '');
    const log = readFileSync(join(home, 'upgrade.log'), 'utf8');
    assert.ok(log.includes('pnpm captured stdout'));
    assert.ok(log.includes('pnpm captured stderr'));
    console.log('Failure log preserves pnpm stdout and stderr.');
  } else if (upgradeHandoff) {
    assert.ok(stdout.includes('Usage:'));
    console.log('The first command runs after self-setup completes.');
  } else {
    assert.deepEqual(
      stdout
        .trim()
        .split('\n')
        .map((line) => line.split('=')[0]),
      ['INSTALL_DIR', 'SHIM_DIR', 'CACHE_DIR', 'CONFIG_DIR', 'STATE_DIR'],
    );
    console.log('Bootstrap stdout contains only shell assignments.');
  }
  console.log('After installation.');
} finally {
  if (interactive) process.stdin.setRawMode(false);
  server.close();
  server.closeAllConnections();
  for (const directory of [home, user, external, resolutionDir])
    rmSync(directory, { recursive: true, force: true });
}
