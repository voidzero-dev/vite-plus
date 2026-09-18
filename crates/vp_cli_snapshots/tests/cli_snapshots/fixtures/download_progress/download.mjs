import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { once } from 'node:events';
import { mkdtempSync, rmSync } from 'node:fs';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { gzipSync } from 'node:zlib';

// Small, valid archives keep this test offline. The installed files are never run.
function archive(files) {
  const blocks = [];
  for (const [name, contents] of Object.entries(files)) {
    const body = Buffer.from(contents);
    const header = Buffer.alloc(512);
    header.write(name);
    header.write('0000755\0', 100);
    header.write(body.length.toString(8).padStart(11, '0') + '\0', 124);
    header.fill(' ', 148, 156);
    header.write('0', 156);
    const checksum = header.reduce((sum, byte) => sum + byte, 0);
    header.write(checksum.toString(8).padStart(6, '0') + '\0 ', 148);
    blocks.push(header, body, Buffer.alloc((512 - (body.length % 512)) % 512));
  }
  return gzipSync(Buffer.concat([...blocks, Buffer.alloc(1024)]));
}

const version = '99.0.0';
const musl = process.platform === 'linux' && !process.report.getReport().header.glibcVersionRuntime;
const platform = `${process.platform}-${process.arch}${musl ? '-musl' : ''}`;
const nodeRoot = `node-v${version}-${platform}`;
const nodeArchive = archive({ [`${nodeRoot}/bin/node`]: '#!/bin/sh\nexit 0\n' });
const npmArchive = archive({
  'package/package.json': JSON.stringify({
    name: 'npm',
    version,
    bin: { npm: 'bin/npm-cli.js', npx: 'bin/npx-cli.js' },
  }),
  'package/bin/npm-cli.js': '// fixture\n',
  'package/bin/npx-cli.js': '// fixture\n',
});
let downloads = 0;
const server = createServer((request, response) => {
  if (request.url.endsWith('/SHASUMS256.txt.asc')) {
    response.writeHead(404).end();
    return;
  }
  if (request.url.endsWith('/SHASUMS256.txt')) {
    response.end(`${createHash('sha256').update(nodeArchive).digest('hex')}  ${nodeRoot}.tar.gz\n`);
    return;
  }
  const body = request.url.endsWith(`/${nodeRoot}.tar.gz`)
    ? nodeArchive
    : request.url === `/npm/-/npm-${version}.tgz`
      ? npmArchive
      : undefined;
  if (!body) {
    response.writeHead(404).end();
    return;
  }
  downloads++;
  if (process.argv[2] === 'known') response.setHeader('Content-Length', body.length);
  response.flushHeaders();
  // Exercise multiple progress redraws; only the completed screen is snapshotted.
  let offset = 0;
  const timer = setInterval(() => {
    const end = Math.min(offset + Math.ceil(body.length / 8), body.length);
    response.write(body.subarray(offset, end));
    offset = end;
    if (offset === body.length) response.end();
  }, 50);
  response.on('close', () => clearInterval(timer));
});
server.listen(0, '127.0.0.1');
await once(server, 'listening');
const mirror = `http://127.0.0.1:${server.address().port}`;
// A separate home prevents mock installs from changing the runner's shared runtime seed.
const home = mkdtempSync(join(tmpdir(), 'vp-download-progress-'));
try {
  for (const tool of ['node', 'npm']) {
    console.log(`Before ${tool} download: preserve this output.`);
    const child = spawn('vp', ['env', 'install', `${tool}@${version}`], {
      stdio: 'inherit',
      env: { ...process.env, VP_HOME: home, VP_NODE_DIST_MIRROR: mirror, npm_config_registry: mirror },
    });
    const [code, signal] = await once(child, 'exit');
    assert.equal(signal, null);
    assert.equal(code, 0);
    console.log(`After ${tool} download.`);
  }
  assert.equal(downloads, 2);
} finally {
  server.close();
  server.closeAllConnections();
  rmSync(home, { recursive: true, force: true });
}
