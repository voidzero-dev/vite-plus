const assert = require('node:assert/strict');
const { realpathSync } = require('node:fs');
const path = require('node:path');

const member = realpathSync('packages/app');
assert.equal(realpathSync(require.resolve('fixture-member/package.json')), path.join(member, 'package.json'));
assert.equal(require('./package.json').dependencies['lifecycle-probe'], 'file:packages/app/lifecycle-probe-1.0.0.tgz');
assert.equal(require('./packages/app/package.json').dependencies?.['lifecycle-probe'], undefined);

const { caller, globalPrefix } = require('lifecycle-probe/install-context.json');
assert.equal(realpathSync(caller), member);
assert.equal(realpathSync(globalPrefix), path.join(member, 'test-global'));
console.log('Root dependencies resolve and the install script retains its caller and global prefix.');
