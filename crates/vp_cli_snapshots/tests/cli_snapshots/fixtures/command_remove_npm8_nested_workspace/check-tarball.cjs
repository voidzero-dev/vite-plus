const assert = require('node:assert/strict');
const { spawnSync } = require('node:child_process');
const { existsSync, readFileSync } = require('node:fs');

const files = ['package.json', 'packages/app/package.json', 'packages/docs/package.json'];
const manifests = () => files.map((file) => readFileSync(file, 'utf8'));
const before = manifests();
const result = spawnSync('vp', ['add', '-w', '--filter', 'docs', './docs-1.0.0.tgz', '--offline'], {
  cwd: 'packages/app',
  encoding: 'utf8',
});
assert.ifError(result.error);
assert.notEqual(result.status, 0);
assert.match(result.stdout + result.stderr, /requires npm 8\.19\.0 or later/);
assert.deepEqual(manifests(), before);
assert.equal(existsSync('package-lock.json'), false);
console.log('npm 8.1 rejected the combined tarball edit before writing manifests or a lockfile.');
