import assert from 'node:assert/strict';
import Module from 'node:module';

const transform = Module._extensions['.ts'];
const resolve = Module._resolveFilename;
assert.equal(typeof transform, 'function');
await import('vite-plus/register');
// Re-evaluate the public wrapper, while retaining the emitted hook identities.
const again = new URL(import.meta.resolve('vite-plus/register'));
again.searchParams.set('again', '1');
await import(again.href);
assert.equal(Module._extensions['.ts'], transform);
assert.equal(Module._resolveFilename, resolve);
const { format } = await import('./module.mts');
assert.equal(format, 'mts');
console.log('registration stays single across repeated and mixed preloads');
