import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';

// The core version encodes the bundled Vite version, so pnpm checks `vite`
// peer ranges against the bundled Vite (rfcs/core-version-alignment.md).
const require = createRequire(import.meta.url);
assert.equal(require('vite/package.json').name, '@voidzero-dev/vite-plus-core');
assert.ok(require.resolve('vite-plugin-current/package.json'));
console.log('pnpm strict peer check accepts a plugin that supports the bundled Vite');

// A plugin that requires an older Vite major must still fail the check.
const project = JSON.parse(readFileSync('package.json', 'utf8'));
project.devDependencies['vite-plugin-legacy'] = 'file:./plugins/legacy';
writeFileSync('package.json', `${JSON.stringify(project, null, 2)}\n`);
const result = spawnSync('vp', ['install', '--ignore-scripts'], {
  encoding: 'utf8',
  timeout: 110_000,
});
assert.ifError(result.error);
const output = result.stdout + result.stderr;
assert.notEqual(result.status, 0, output);
assert.match(output, /ERR_PNPM_PEER_DEP_ISSUES/);
assert.match(output, /vite-plugin-legacy/);
assert.doesNotMatch(output, /vite-plugin-current/);
console.log('pnpm strict peer check rejects a plugin that requires Vite 7');
