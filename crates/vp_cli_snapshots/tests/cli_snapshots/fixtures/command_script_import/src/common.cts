const assert = require('node:assert/strict');
enum Format { Value = 'cts' }
const nested: string = require('./nested.cts');
assert.equal(nested, 'nested');
module.exports = { format: Format.Value };
