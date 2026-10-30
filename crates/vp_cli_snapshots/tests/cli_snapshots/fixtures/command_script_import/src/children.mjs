import { fork } from 'node:child_process';
import { once } from 'node:events';
import { Worker } from 'node:worker_threads';

async function collect(child) {
  const signal = AbortSignal.timeout(15000);
  const exited = once(child, 'exit', { signal });
  const [message] = await once(child, 'message', { signal });
  const [exitCode] = await exited;
  return { message, exitCode };
}
const forked = await collect(fork(new URL('./child.ts', import.meta.url)));
const esmWorker = await collect(new Worker(new URL('./worker.mts', import.meta.url)));
const cjsWorker = await collect(new Worker(new URL('./worker.cts', import.meta.url)));
console.log(JSON.stringify({ fork: forked, esmWorker, cjsWorker }, null, 2));
