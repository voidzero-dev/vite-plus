// Some Node.js mirrors, such as mirrors.aliyun.com/nodejs-release, return 403
// to requests without a User-Agent header (#3422). This mock mirror does the
// same and records the User-Agent of each request.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { once } from 'node:events';
import { mkdtempSync, rmSync } from 'node:fs';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createArchive } from './archive.mjs';

const version = '99.0.0';
const musl = process.platform === 'linux' && !process.report.getReport().header.glibcVersionRuntime;
const platform = `${process.platform}-${process.arch}${musl ? '-musl' : ''}`;
const nodeRoot = `node-v${version}-${platform}`;
const nodeArchive = createArchive({ [`${nodeRoot}/bin/node`]: '#!/bin/sh\nexit 0\n' });
const userAgents = [];
const server = createServer(function handleRequest(request, response) {
  const userAgent = request.headers['user-agent'];
  userAgents.push(userAgent);
  if (!userAgent) {
    response.writeHead(403).end();
    return;
  }
  if (request.url.endsWith('/SHASUMS256.txt.asc')) {
    response.writeHead(404).end();
    return;
  }
  if (request.url.endsWith('/SHASUMS256.txt')) {
    response.end(`${createHash('sha256').update(nodeArchive).digest('hex')}  ${nodeRoot}.tar.gz\n`);
    return;
  }
  if (request.url.endsWith(`/${nodeRoot}.tar.gz`)) {
    response.end(nodeArchive);
    return;
  }
  response.writeHead(404).end();
});
server.listen(0, '127.0.0.1');
await once(server, 'listening');
const mirror = `http://127.0.0.1:${server.address().port}`;
// A separate home prevents mock installs from changing the runner's shared runtime seed.
const home = mkdtempSync(join(tmpdir(), 'vp-download-user-agent-'));
try {
  const child = spawn('vp', ['env', 'install', `node@${version}`], {
    stdio: 'inherit',
    env: { ...process.env, VP_HOME: home, VP_NODE_DIST_MIRROR: mirror },
  });
  const [code, signal] = await once(child, 'exit');
  assert.equal(signal, null);
  assert.equal(code, 0);
  assert.ok(userAgents.length > 0);
  for (const userAgent of userAgents) {
    assert.match(userAgent ?? '', /^vp\/\S+$/);
  }
  console.log('Each request sent a vp User-Agent.');
} finally {
  server.close();
  server.closeAllConnections();
  rmSync(home, { recursive: true, force: true });
}
