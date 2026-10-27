import assert from 'node:assert/strict';
import { readFileSync, writeFileSync } from 'node:fs';
import { basename } from 'node:path';

assert.equal(readFileSync('resolved.txt', 'utf8'), 'configResolved passed\n');
assert.equal(process.argv.length, 3);
assert.equal(basename(process.argv[2]), 'input.txt');
writeFileSync('result.txt', 'staged command from plugin passed\n');
