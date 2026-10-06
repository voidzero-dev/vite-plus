const assert = require('node:assert/strict');
const { readFileSync } = require('node:fs');

assert.deepEqual(process.argv.slice(2), ['--flag', 'value with spaces', 'path\\file']);
assert.equal(process.env.SHIM_TEST_VALUE, 'inherited value');
assert.equal(process.versions.node, readFileSync('.node-version', 'utf8'));
console.log('arguments, environment, cwd, and Node selection preserved');
process.exit(Number(process.env.SHIM_TEST_EXIT_CODE || 0));
