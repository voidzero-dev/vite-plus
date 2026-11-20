const assert = require('node:assert/strict');
const { readFileSync } = require('node:fs');

const { name } = JSON.parse(readFileSync('package.json', 'utf8'));
const args = process.argv.slice(2);
assert.equal(name, 'app');
assert.equal(process.env.VP_EXPLICIT_CHDIR, '1');
assert.deepEqual(args, ['build']);
console.log(`Global CLI in ${name}: ${JSON.stringify(args)}`);
