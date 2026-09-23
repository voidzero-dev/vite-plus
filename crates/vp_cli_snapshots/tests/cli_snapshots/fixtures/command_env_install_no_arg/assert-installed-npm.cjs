const assert = require('node:assert/strict');
const { execFileSync } = require('node:child_process');
const manifest = require('./mock-manifest.json');

const installed = JSON.parse(
  execFileSync('vp', ['env', 'list', 'npm', '--json'], { encoding: 'utf8' }),
).package_managers.npm;
assert.equal(installed.length, 1);
assert.equal(installed[0].version, manifest['npm/latest'].version);
assert.equal(installed[0].current, false);
assert.equal(installed[0].default, false);
console.log('Standalone npm is installed but is not current');
