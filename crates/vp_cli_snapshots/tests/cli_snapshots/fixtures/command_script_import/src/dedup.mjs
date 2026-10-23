import Module from 'node:module';

const transform = Module._extensions['.ts'];
const resolve = Module._resolveFilename;
await import('vite-plus/register');
// Re-evaluate the public wrapper, while retaining the emitted hook identities.
const again = new URL(import.meta.resolve('vite-plus/register'));
again.searchParams.set('again', '1');
await import(again.href);
const { format } = await import('./module.mts');
console.log(JSON.stringify({
  transformType: typeof transform,
  sameTransform: Module._extensions['.ts'] === transform,
  sameResolve: Module._resolveFilename === resolve,
  format,
}, null, 2));
