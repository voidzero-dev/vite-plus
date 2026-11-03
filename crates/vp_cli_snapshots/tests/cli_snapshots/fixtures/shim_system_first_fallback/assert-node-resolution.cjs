const assert = require('node:assert/strict');
const { execFileSync } = require('node:child_process');
const { realpathSync } = require('node:fs');

const output = execFileSync('vp', ['env', 'which', 'node'], { encoding: 'utf8', timeout: 10000 });
assert.equal(realpathSync(output.split('\n')[0].trim()), realpathSync(process.execPath));
console.log('env which selects the same Node as PATH');
