import { parentPort } from 'node:worker_threads';

enum Source {
  Worker = 'enum from worker.ts',
}
parentPort?.postMessage(Source.Worker);
