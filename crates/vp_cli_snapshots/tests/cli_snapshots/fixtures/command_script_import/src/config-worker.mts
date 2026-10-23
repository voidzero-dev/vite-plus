import { once } from 'node:events';
import { createRequire } from 'node:module';
import { parentPort } from 'node:worker_threads';
import { value as initial } from '@value';

enum Stage { Ready = 'ready', Result = 'result' }
const run = once(parentPort!, 'message');
parentPort!.postMessage({ stage: Stage.Ready, initial });
await run;
// The other worker has registered before either worker resolves or transforms these.
const { value } = await import('./config-parent-check.mts');
const { writes } = createRequire(import.meta.url)('./config-fields.cts');
parentPort!.postMessage({ stage: Stage.Result, value, writes });
parentPort!.close();
