import assert from 'node:assert/strict';
import { appendFileSync, readFileSync } from 'node:fs';
import { createRequire, getCompileCacheDir } from 'node:module';
import path from 'node:path';

import { Value } from './value.ts';

const require = createRequire(import.meta.url);
const legacy = require('./legacy.cts');
const preload = process.execArgv.find((arg) => path.basename(arg) === 'script-preload.cjs');
assert.ok(preload, 'the CLI must preload the selected loader');
const packageRoot = path.dirname(path.dirname(preload));
const { oxcNode } = require(path.join(packageRoot, 'binding/index.cjs'));

function h(tag: string): string {
  return `h:${tag}`;
}
function j(tag: string): string {
  return `j:${tag}`;
}

const [value, factory, state] = process.argv.slice(2);
assert.equal(Value.Current, value);
assert.equal(legacy, value);
assert.equal(<entry />, `${factory}:entry`);
const stats = oxcNode.cacheStats();
if (state === 'warm') {
  assert.ok(stats.hits > 0, 'warm launch must reuse a transform');
  assert.equal(stats.misses, 0, 'unchanged transforms must not run again');
  assert.equal(stats.errors, 0, 'warm records must be readable');
} else if (state === 'cold' || state === 'edited') {
  assert.ok(stats.misses > 0, 'new sources must be transformed');
  assert.ok(stats.writes > 0, 'successful transforms must be persisted');
} else if (state === 'repaired') {
  assert.ok(stats.errors > 0, 'corrupt records must be rejected');
  assert.ok(stats.writes > 0, 'corrupt records must be replaced');
} else if (state === 'disabled') {
  assert.deepEqual(stats, { hits: 0, misses: 0, writes: 0, errors: 0 });
}
if (process.env.VPX_CACHE === '1' && state !== 'disabled') {
  assert.ok(getCompileCacheDir(), 'the script process must enable its compile cache');
}

// Count a visible side effect separately from transform hits. Every invocation runs.
appendFileSync('effects.txt', `${value}\n`);
const runs = readFileSync('effects.txt', 'utf8').trim().split('\n').length;
console.log(`cache ${state}: esm=${Value.Current} cjs=${legacy} jsx=${factory} run=${runs}`);
