const assert = require('node:assert/strict');
const pkg = require('./package.json');

assert.equal(pkg.packageManager, undefined);
assert.equal(pkg.devEngines?.packageManager, undefined);
console.log('Migration leaves npm unpinned');
