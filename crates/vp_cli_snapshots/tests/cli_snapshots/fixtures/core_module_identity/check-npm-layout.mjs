import assert from 'node:assert/strict';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const cliRequire = createRequire(require.resolve('vite-plus/package.json'));
const vitestRequire = createRequire(cliRequire.resolve('vitest/package.json'));
const project = require('./package.json');
assert.equal(project.overrides, undefined);
assert.equal(cliRequire('vite/package.json').name, '@voidzero-dev/vite-plus-core');
if (project.devDependencies.vite) {
  assert.equal(
    project.devDependencies.vite,
    `npm:@voidzero-dev/vite-plus-core@${cliRequire('vite-plus/package.json').version}`,
  );
  assert.equal(require.resolve('vite/package.json'), cliRequire.resolve('vite/package.json'));
  assert.equal(vitestRequire.resolve('vite/package.json'), cliRequire.resolve('vite/package.json'));
  console.log(
    'npm shares the exact core alias between the project, CLI, and Vitest without overrides',
  );
} else {
  assert.deepEqual(Object.keys(project.devDependencies), ['vite-plus']);
  assert.equal(require('vite/package.json').name, 'vite');
  assert.equal(vitestRequire('vite/package.json').name, 'vite');
  assert.notEqual(
    cliRequire.resolve('vite/package.json'),
    vitestRequire.resolve('vite/package.json'),
  );
  console.log('npm installed separate CLI core and upstream Vitest peer without overrides');
}
