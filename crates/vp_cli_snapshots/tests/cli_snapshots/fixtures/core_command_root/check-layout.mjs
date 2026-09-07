import assert from 'node:assert/strict';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const appRequire = createRequire(new URL('./packages/app/package.json', import.meta.url));
const cliRequire = createRequire(require.resolve('vite-plus/package.json'));
console.log('Root Vite:', require('vite/package.json').name);
console.log('App Vite:', appRequire('vite/package.json').name);
console.log('CLI Vite:', cliRequire('vite/package.json').name);
assert.equal(require('vite/package.json').name, 'vite');
assert.equal(appRequire('vite/package.json').name, '@voidzero-dev/vite-plus-core');
assert.equal(cliRequire('vite/package.json').name, '@voidzero-dev/vite-plus-core');
