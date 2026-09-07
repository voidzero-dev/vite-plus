import assert from 'node:assert/strict';
import { createRequire } from 'node:module';

import * as vite from 'vite';
import { ModuleRunner } from 'vite/module-runner';
import * as vp from 'vite-plus';
import { ModuleRunner as VpModuleRunner } from 'vite-plus/module-runner';
import pkg from 'vite-plus/package.json' with { type: 'json' };

const require = createRequire(import.meta.url);
const cjsVite = require('vite');
const cjsVp = require('vite-plus');
for (const name of [
  'createServer',
  'DevEnvironment',
  'mergeConfig',
  'isRunnableDevEnvironment',
  'isFetchableDevEnvironment',
]) {
  assert.equal(vite[name], vp[name], name);
  assert.equal(cjsVite[name], cjsVp[name], name);
  assert.equal(vite[name], cjsVite[name], name);
}
assert.equal(ModuleRunner, VpModuleRunner);
const config = {};
assert.equal(vite.defineConfig(config), config);
assert.equal(pkg.dependencies.vite, `npm:@voidzero-dev/vite-plus-core@${pkg.version}`);
assert.equal(pkg.dependencies['@voidzero-dev/vite-plus-core'], undefined);
console.log('Packed alias, ESM, CommonJS, and module-runner identity passed');
