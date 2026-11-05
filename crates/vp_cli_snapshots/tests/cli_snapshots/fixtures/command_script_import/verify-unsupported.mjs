import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';

assert.equal(Number(process.versions.node.split('.')[0]), 20);
const child = spawnSync(process.execPath, [
  '--require', './binding-guard.cjs',
  '--import=vite-plus/register',
  '--eval', 'throw new Error("APPLICATION_RAN");',
], { encoding: 'utf8', timeout: 15000 });
assert.equal(child.error, undefined);
assert.equal(child.status, 1, child.stderr);
assert.equal(child.stdout, '');
assert.match(child.stderr, /requires Node\.js/);
assert.doesNotMatch(child.stderr, /NATIVE_BINDING_LOADED|APPLICATION_RAN|SyntaxError/);
console.log('unsupported Node.js fails before binding and application loading');
