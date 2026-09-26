import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { randomBytes } from 'node:crypto';
import { once } from 'node:events';
import { copyFileSync, mkdirSync, readFileSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { join, resolve } from 'node:path';

import { createArchive } from './archive.mjs';

const interactive = process.argv.includes('interactive');
const failure = process.argv.includes('failure');
if (interactive) process.stdin.setRawMode(true);
const home = resolve('home');
const user = resolve('user');
const external = resolve('external');
const nodeBin = join(home, 'js_runtime/node/99.0.0/bin');
for (const directory of [nodeBin, user, external]) mkdirSync(directory, { recursive: true });
// Use the runner's real Node for the mock pnpm program, without a network download.
symlinkSync(process.execPath, join(nodeBin, 'node'));
// Model Node's bundled npm while retaining deterministic download checkpoints.
const npmBin = join(nodeBin, '../lib/node_modules/npm/bin');
mkdirSync(npmBin, { recursive: true });
writeFileSync(
  join(npmBin, 'npm-cli.js'),
  `
  const fs = require('node:fs');
  fetch(process.env.npm_config_registry + '/pnpm/-/pnpm-10.33.0.tgz')
    .then(response => response.arrayBuffer())
    .then(data => {
      const filename = 'pnpm-10.33.0.tgz';
      fs.writeFileSync(filename, Buffer.from(data));
      console.log(JSON.stringify([{ name: 'pnpm', version: '10.33.0', filename }]));
    });
`,
);
copyFileSync(join(process.env.VP_HOME, 'bin/vp'), join(external, 'vp'));

const pnpmArchive = createArchive({
  'package/package.json': JSON.stringify({
    name: 'pnpm',
    version: '10.33.0',
    bin: { pnpm: 'bin/pnpm.cjs' },
  }),
  'package/bin/pnpm.cjs': `
    const fs = require('node:fs');
    console.log('pnpm captured stdout');
    console.error('pnpm captured stderr');
    fetch(process.env.npm_config_registry + '/install').then(response => {
      if (!response.ok) process.exit(17);
      fs.mkdirSync('node_modules/vite-plus', { recursive: true });
      fs.writeFileSync('node_modules/vite-plus/package.json', '{"name":"vite-plus"}');
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
  process.stderr.write(`\x1b]2;pty-terminal-test:${id}:${name}\x1b\\`);
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
    response.writeHead(failure ? 500 : 200).end();
  } else {
    response.writeHead(404).end();
  }
}

const server = createServer(handleRequest);
server.listen(0, '127.0.0.1');
await once(server, 'listening');
const mirror = `http://127.0.0.1:${server.address().port}`;

try {
  console.log('Before installation: preserve this output.');
  const child = spawn(join(external, 'vp'), [], {
    stdio: ['ignore', 'pipe', 'inherit'],
    env: {
      ...process.env,
      HOME: user,
      PATH: nodeBin,
      VP_HOME: home,
      VP_NODE_DIST_MIRROR: mirror,
      npm_config_registry: mirror,
      NPM_CONFIG_REGISTRY: mirror,
      VP_SELF_SETUP_SHELL: 'sh',
      VP_SELF_SETUP_NO_MODIFY_PATH: '1',
      VP_NODE_MANAGER: 'no',
      VP_SHELL: 'zsh',
      VP_SKIP_DEPS_INSTALL: '',
    },
  });
  let stdout = '';
  child.stdout.setEncoding('utf8').on('data', (chunk) => {
    stdout += chunk;
  });
  const [code, signal] = await once(child, 'exit');
  assert.equal(signal, null);
  assert.equal(code, failure ? 1 : 0);
  assert.deepEqual(phases, ['prepare', 'download', 'dependencies']);
  if (failure) {
    assert.equal(stdout, '');
    const log = readFileSync(join(home, 'upgrade.log'), 'utf8');
    assert.ok(log.includes('exit code 17'));
    assert.ok(!log.includes('pnpm captured'));
    console.log('Failure log records the exit code without registry output.');
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
  for (const directory of [home, user, external])
    rmSync(directory, { recursive: true, force: true });
}
