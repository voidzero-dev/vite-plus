import { once } from 'node:events';
import { createRequire } from 'node:module';
import { pathToFileURL } from 'node:url';
import { Worker } from 'node:worker_threads';

const register = pathToFileURL(createRequire(import.meta.url).resolve('vite-plus/register')).href;
const worker = new Worker(new URL('./worker.mts', import.meta.url), {
  execArgv: ['--import', register],
});
try {
  const [message] = await once(worker, 'message', { signal: AbortSignal.timeout(15000) });
  console.log(JSON.stringify({ message }, null, 2));
} finally {
  await worker.terminate();
}
