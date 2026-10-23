import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';

assert.deepEqual(process.execArgv, []);
const child = spawnSync(process.execPath, ['./src/main.ts'], {
  encoding: 'utf8',
  timeout: 15000,
});
assert.equal(child.error, undefined);
assert.equal(child.status, 0, child.stderr);
assert.equal(child.stdout.trim(), 'esm syntax, resolution, JSX, helpers, and dependencies');
console.log('spawn inherits NODE_OPTIONS registration');
