import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

import { app } from './dist/index.mjs';

assert.equal(app, 'fixture');
assert.equal(readFileSync('resolved.txt', 'utf8'), 'configResolved passed\n');
console.log('pack command from plugin passed');
