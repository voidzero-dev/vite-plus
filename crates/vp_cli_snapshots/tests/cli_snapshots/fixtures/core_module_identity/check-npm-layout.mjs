import assert from 'node:assert/strict';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const cliRequire = createRequire(require.resolve('vite-plus/package.json'));
const vitestRequire = createRequire(cliRequire.resolve('vitest/package.json'));
assert.equal(require('./package.json').overrides, undefined);
assert.equal(cliRequire('vite/package.json').name, '@voidzero-dev/vite-plus-core');
assert.equal(vitestRequire('vite/package.json').name, 'vite');
assert.notEqual(cliRequire.resolve('vite/package.json'), vitestRequire.resolve('vite/package.json'));
if (!require('./package.json').devDependencies.vite) {
  assert.deepEqual(Object.keys(require('./package.json').devDependencies), ['vite-plus']);
  assert.equal(require('vite/package.json').name, 'vite');
}
console.log('npm installed separate CLI core and upstream Vitest peer without overrides');
