import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { connect } from 'node:net';

import { failAndStopGroup, linesWithin, parseListening } from './helpers.mjs';

const workerMode = process.argv[2] === '--worker';

async function listenerInfo(stdout, launchPid) {
  let listener;
  for await (const line of linesWithin(stdout)) {
    listener = parseListening(line) ?? listener;
    if (listener && !workerMode) {
      assert.equal(listener.pid, launchPid, 'Vite runs in the launcher process');
      return listener;
    }
    if (listener && line === 'worker host alive') {
      assert.notEqual(listener.pid, launchPid, 'Vite replaced the Worker host');
      return listener;
    }
  }
  assert.fail('Vite did not listen in the expected process');
}

const args = ['dev', '--host', '127.0.0.1', '--port', '0', '--base', '/probe/'];
const child = spawn(
  workerMode ? process.execPath : 'vp',
  workerMode ? ['worker-host.mjs', ...args] : args,
  { stdio: ['ignore', 'pipe', 'inherit'] },
);

try {
  const { port, pid } = await listenerInfo(child.stdout, child.pid);
  const response = await fetch(`http://127.0.0.1:${port}/probe/`, {
    signal: AbortSignal.timeout(3_000),
  });
  assert.equal(response.status, 200);
  const html = await response.text();
  assert.match(html, /Project-local Vite/);
  assert.match(html, /src="\/probe\/@vite\/client"/);
  console.log('Vite served the project with the client under /probe/');
  if (workerMode) {
    console.log('The Worker host stayed alive while Vite ran as a child');
  }

  process.kill(pid, 'SIGTERM');
  const [code, signal] = await once(child, 'exit', { signal: AbortSignal.timeout(5_000) });
  const socket = connect(port, '127.0.0.1');
  try {
    await assert.rejects(once(socket, 'connect', { signal: AbortSignal.timeout(3_000) }), {
      code: 'ECONNREFUSED',
    });
  } finally {
    socket.destroy();
  }
  assert.deepEqual([code, signal], [143, null]);
  const target = workerMode ? 'Vite child' : 'launch PID';
  console.log(`SIGTERM to the ${target} closed the port with exit code 143`);
} catch (error) {
  failAndStopGroup(error);
}
