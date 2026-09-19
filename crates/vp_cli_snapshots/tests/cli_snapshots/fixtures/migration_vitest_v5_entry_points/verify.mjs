import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';

import * as entries from './entries.mjs';

assert.equal(typeof entries.BaseCoverageProvider, 'function');
assert.equal(typeof entries.DefaultReporter, 'function');
assert.equal(typeof entries.environments.populateGlobal, 'function');
assert.equal(typeof entries.VitestSnapshotEnvironment, 'function');
assert.ok(Object.keys(entries.mocker).length > 0);
assert.equal((await entries.loadEnvironment()).populateGlobal, entries.environments.populateGlobal);

const require = createRequire(import.meta.url);
const source = readFileSync(new URL('./entries.mjs', import.meta.url), 'utf8');
for (const name of ['coverage', 'reporters', 'environments', 'snapshot']) {
  const specifier = `vite-plus/test/${name}`;
  assert.ok(!source.includes(specifier), `Unmigrated import: ${specifier}`);
  assert.throws(() => require.resolve(specifier), { code: 'ERR_PACKAGE_PATH_NOT_EXPORTED' });
}
console.log('Canonical entry points load; legacy aliases are absent; mocker remains available.');
