import assert from 'node:assert/strict';
import { createRequire } from 'node:module';

import { createServer } from 'vite-plus';
import { build } from 'vite-plus/pack';

const require = createRequire(import.meta.url);
const cliRequire = createRequire(require.resolve('vite-plus/package.json'));
assert.equal(createServer, cliRequire('vite').createServer);
assert.equal(build, cliRequire('vite/pack').build);
assert.equal(require('./package.json').devDependencies.vite, undefined);
console.log('Bundled APIs resolve without a project Vite dependency');
