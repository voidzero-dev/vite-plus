import assert from 'node:assert/strict';
import { fork } from 'node:child_process';
import { once } from 'node:events';
import { Worker } from 'node:worker_threads';

async function collect(child, expected) {
  const signal = AbortSignal.timeout(15000);
  const exited = once(child, 'exit', { signal });
  const [message] = await once(child, 'message', { signal });
  assert.equal(message, expected);
  const [code] = await exited;
  assert.equal(code, 0);
}
const expected = process.env.EXPECTED_CONFIG_VALUE ?? 'default';
await collect(fork(new URL('./child.ts', import.meta.url)), `fork: ${expected}`);
await collect(new Worker(new URL('./worker.mts', import.meta.url)), `esm worker: cts: ${expected}`);
await collect(new Worker(new URL('./worker.cts', import.meta.url)), 'cjs worker: cts');
console.log('fork and ESM/CJS workers inherit registration');
