import assert from 'node:assert/strict';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const cliRequire = createRequire(require.resolve('vite-plus/package.json'));
const vitestRequire = createRequire(cliRequire.resolve('vitest/package.json'));
const project = require('./package.json');
assert.equal(project.overrides, undefined);
assert.equal(cliRequire('vite/package.json').name, '@voidzero-dev/vite-plus-core');
assert.equal(vitestRequire('vite/package.json').name, 'vite');
assert.notEqual(cliRequire.resolve('vite/package.json'), vitestRequire.resolve('vite/package.json'));
if (!project.devDependencies.vite) {
  assert.deepEqual(Object.keys(project.devDependencies), ['vite-plus']);
  assert.equal(require('vite/package.json').name, 'vite');
}
console.log('npm installed separate CLI core and upstream Vitest peer without overrides');
