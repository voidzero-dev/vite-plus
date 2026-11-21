import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { existsSync, mkdtempSync, rmSync } from 'node:fs';
import { connect } from 'node:net';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createInterface } from 'node:readline';

const workerMode = process.argv[2] === '--worker';
// The global vp runs `dist/bin.js`, which keeps Vite in a child process.
const globalMode = process.argv[2] === '--global';
const localMode = !workerMode && !globalMode;

async function listenerInfo(stdout, launchPid) {
  let listener;
  for await (const line of createInterface({ input: stdout })) {
    const match = /^listening (\d+) (\d+)$/.exec(line);
    if (match) {
      listener = { port: Number(match[1]), pid: Number(match[2]) };
      if (globalMode) {
        assert.notEqual(listener.pid, launchPid, 'Vite runs below the global vp process');
        return listener;
      }
      if (!workerMode) {
        assert.equal(listener.pid, launchPid);
        return listener;
      }
      assert.notEqual(listener.pid, launchPid, 'Vite replaced the Worker host');
    }
    if (listener && line === 'worker host alive') {
      return listener;
    }
  }
  assert.fail('Vite did not listen in the expected process');
}

const cache = mkdtempSync(join(tmpdir(), 'vp-dev-cache-'));
const cacheProof = join(cache, 'cache-before-vite');
const env = { ...process.env, NODE_COMPILE_CACHE: cache };
delete env.NODE_DISABLE_COMPILE_CACHE;
if (localMode) {
  env.NODE_OPTIONS = ['--require=./check-launcher-cache.cjs', env.NODE_OPTIONS]
    .filter(Boolean)
    .join(' ');
  env.VP_PROBE_CACHE_PROOF = cacheProof;
}
const args = ['dev', '--host', '127.0.0.1', '--port', '0', '--base', '/probe/'];
const child = spawn(
  workerMode ? process.execPath : 'vp',
  workerMode ? ['worker-host.mjs', ...args] : args,
  { stdio: ['ignore', 'pipe', 'inherit'], env },
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
  } else if (localMode) {
    assert.ok(
      existsSync(cacheProof),
      'Vite startup checked the launcher cache before loading its entry point',
    );
    console.log('vp saved its compile cache before starting Vite');
  }

  process.kill(globalMode ? child.pid : pid, 'SIGTERM');
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
  const target = workerMode ? 'Vite child' : globalMode ? 'global vp PID' : 'launch PID';
  console.log(`SIGTERM to the ${target} closed the port with exit code 143`);
} catch (error) {
  console.error(error);
  process.exitCode = 1;
} finally {
  rmSync(cache, { recursive: true, force: true });
  if (process.exitCode === 1) {
    // The runner starts the probe in its own group. Stop its descendants
    // before an early failure can leave the runner waiting for their pipes.
    process.kill(-process.pid, 'SIGKILL');
  }
}
