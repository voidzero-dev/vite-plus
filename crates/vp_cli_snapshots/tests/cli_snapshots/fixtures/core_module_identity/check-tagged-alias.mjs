import assert from 'node:assert/strict';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const cliRequire = createRequire(require.resolve('vite-plus/package.json'));
const vitestRequire = createRequire(cliRequire.resolve('vitest/package.json'));
const project = require('./package.json');
assert.equal(project.overrides, undefined);
assert.equal(project.devDependencies.vite, 'npm:@voidzero-dev/vite-plus-core@latest');
// The core version encodes the bundled Vite version, so it satisfies Vitest's
// `vite` peer range and npm keeps the tagged alias on the core
// (rfcs/core-version-alignment.md). A `1.x` core made npm replace the alias
// with upstream Vite.
assert.equal(require('vite/package.json').name, '@voidzero-dev/vite-plus-core');
assert.equal(require.resolve('vite/package.json'), cliRequire.resolve('vite/package.json'));
assert.equal(vitestRequire.resolve('vite/package.json'), cliRequire.resolve('vite/package.json'));
console.log('npm keeps the tagged core alias and shares it between the project, CLI, and Vitest');
