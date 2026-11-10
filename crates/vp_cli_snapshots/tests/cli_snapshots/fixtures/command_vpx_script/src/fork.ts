// Child processes and workers inherit the loader through `process.execArgv`.
import { fork } from 'node:child_process';
import { once } from 'node:events';
import { Worker } from 'node:worker_threads';

const child = fork(new URL('./child.ts', import.meta.url));
const [fromChild] = await once(child, 'message');
console.log('fork:', fromChild);

const worker = new Worker(new URL('./worker.ts', import.meta.url));
const [fromWorker] = await once(worker, 'message');
console.log('worker:', fromWorker);
await worker.terminate();
