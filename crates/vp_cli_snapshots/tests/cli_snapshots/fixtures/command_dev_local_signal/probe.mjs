import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdtempSync, readdirSync, rmSync } from 'node:fs';
import { connect } from 'node:net';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createInterface } from 'node:readline';

async function listeningPort(stdout) {
  for await (const line of createInterface({ input: stdout })) {
    const match = /^listening (\d+)$/.exec(line);
    if (match) {
      return Number(match[1]);
    }
  }
  assert.fail('Vite exited before listening');
}

const cache = mkdtempSync(join(tmpdir(), 'vp-dev-cache-'));
const env = { ...process.env, NODE_COMPILE_CACHE: cache };
delete env.NODE_DISABLE_COMPILE_CACHE;
const child = spawn('vp', ['dev', '--host', '127.0.0.1', '--port', '0', '--base', '/probe/'], {
  detached: true,
  stdio: ['ignore', 'pipe', 'inherit'],
  env,
});
const exited = once(child, 'exit');
function stopProcessGroup() {
  if (child.pid !== undefined) {
    try {
      process.kill(-child.pid, 'SIGKILL');
    } catch (err) {
      if (err.code !== 'ESRCH') {
        throw err;
      }
    }
  }
}

const timer = setTimeout(stopProcessGroup, 20_000);
try {
  const port = await listeningPort(child.stdout);
  const response = await fetch(`http://127.0.0.1:${port}/probe/`, {
    signal: AbortSignal.timeout(3_000),
  });
  assert.equal(response.status, 200);
  const html = await response.text();
  assert.match(html, /Project-local Vite/);
  assert.match(html, /src="\/probe\/@vite\/client"/);
  console.log('Vite served the project with the client under /probe/');
  assert.ok(
    readdirSync(cache, { recursive: true, withFileTypes: true }).some((entry) => entry.isFile()),
    'vp saves its compile cache before starting Vite',
  );
  console.log('vp saved its compile cache before starting Vite');

  child.kill('SIGTERM');
  const [code, signal] = await exited;
  const socket = connect(port, '127.0.0.1');
  try {
    await assert.rejects(once(socket, 'connect', { signal: AbortSignal.timeout(3_000) }), {
      code: 'ECONNREFUSED',
    });
  } finally {
    socket.destroy();
  }
  assert.deepEqual([code, signal], [143, null]);
  console.log('SIGTERM to the launch PID closed the port with exit code 143');
} finally {
  clearTimeout(timer);
  stopProcessGroup();
  rmSync(cache, { recursive: true, force: true });
}
