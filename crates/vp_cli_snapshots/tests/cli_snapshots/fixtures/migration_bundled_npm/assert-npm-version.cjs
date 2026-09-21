const assert = require('node:assert/strict');
const { dirname, resolve } = require('node:path');

const bundled = require(resolve(
  dirname(process.execPath),
  process.platform === 'win32' ? 'node_modules/npm/package.json' : '../lib/node_modules/npm/package.json',
)).version;
const pkg = require('./package.json');
assert.equal(pkg.devEngines.packageManager.name, 'npm');
assert.equal(pkg.devEngines.packageManager.version, bundled);
console.log('Migration pins the bundled npm version');
