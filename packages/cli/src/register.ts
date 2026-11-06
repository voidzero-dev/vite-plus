/**
 * Public `node --import vite-plus/register` entry. Reuse the emitted loader files
 * so Node.js caches registration together with the two flags passed by `vpx`.
 */
import { createRequire } from 'node:module';
import * as workerThreads from 'node:worker_threads';

// `NODE_OPTIONS` can also preload this module in Node.js' own loader worker.
// Registering another loader there would recursively create loader workers.
// Namespace lookup also lets unsupported older runtimes reach the preload's
// version diagnostic instead of failing to link a missing named export.
if (!workerThreads.isInternalThread) {
  createRequire(import.meta.url)('./script-preload.cjs');
  // Keep this as a runtime URL: bundling script-register into this entry would
  // register its hooks again when the CLI also preloads script-register.js.
  await import(new URL('./script-register.js', import.meta.url).href);
}
