import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const cliRequire = createRequire(require.resolve('vite-plus/package.json'));
const project = require('./package.json');
assert.equal(project.overrides, undefined);
assert.equal(project.devDependencies.vite, 'npm:@voidzero-dev/vite-plus-core@latest');
// npm 11.11 accepts a peer's upstream Vite as satisfying the tagged alias.
assert.equal(require('vite/package.json').name, 'vite');
assert.equal(cliRequire('vite/package.json').name, '@voidzero-dev/vite-plus-core');
assert.notEqual(require.resolve('vite/package.json'), cliRequire.resolve('vite/package.json'));

for (const args of [['dev'], ['pack', 'entry.js']]) {
  const result = spawnSync('vp', args, { encoding: 'utf8', timeout: 30_000 });
  assert.ifError(result.error);
  const output = result.stdout + result.stderr;
  assert.equal(result.status, 1, output);
  assert.match(output, /Expected @voidzero-dev\/vite-plus-core@/);
  assert.match(output, /but found vite@/);
  assert.match(output, /Run `vp migrate` to align the Vite alias, then run `vp install`\./);
  console.log(`vp ${args[0]} rejects the replaced npm alias and reports the repair command`);
}
