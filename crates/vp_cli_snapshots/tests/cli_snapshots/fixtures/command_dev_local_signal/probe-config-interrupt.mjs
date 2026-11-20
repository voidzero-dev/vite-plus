import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { on, once } from 'node:events';
import { createInterface } from 'node:readline';

async function stopDuringConfig(signal) {
  const child = spawn('vp', ['dev', '--host', '127.0.0.1', '--port', '0'], {
    env: { ...process.env, VP_PROBE_SYNC_CONFIG: '1' },
    stdio: ['inherit', 'pipe', 'inherit'],
  });
  const lines = createInterface({ input: child.stdout });
  try {
    for await (const [line] of on(lines, 'line', {
      signal: AbortSignal.timeout(10_000),
      close: ['close'],
    })) {
      if (line === 'Evaluating Vite config') {
        child.kill(signal);
        const termination = await once(child, 'exit', { signal: AbortSignal.timeout(3_000) });
        assert.deepEqual(termination, [null, signal]);
        console.log(`${signal} stopped Vite during synchronous config evaluation`);
        return;
      }
    }
    assert.fail('Vite did not start evaluating the config');
  } finally {
    lines.close();
    if (child.exitCode === null && child.signalCode === null) {
      child.kill('SIGKILL');
    }
  }
}

await stopDuringConfig('SIGINT');
await stopDuringConfig('SIGTERM');
