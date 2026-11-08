import Module from 'node:module';
import { isInternalThread } from 'node:worker_threads';

export async function initialize({ port }) {
  const transform = Module._extensions['.ts'];
  const resolve = Module._resolveFilename;
  await import('vite-plus/register');
  port.postMessage({
    internal: isInternalThread,
    unchanged: Module._extensions['.ts'] === transform && Module._resolveFilename === resolve,
  });
  port.close();
}
