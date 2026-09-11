const assert = require('node:assert/strict');
const { execSync } = require('node:child_process');

assert.equal(process.version, 'v22.18.0');
assert.equal(execSync('npm --version', { encoding: 'utf8' }).trim(), '10.5.0');
console.log('JS delegation preserves the explicit Node and npm versions');
