// tsconfig `paths`, `.js` -> `.ts`, extensionless imports, JSON, and CommonJS.
import { createRequire } from 'node:module';

import { greet } from '@lib/greet';

import data from './data.json' with { type: 'json' };
import { double } from './math.js';
import { triple } from './math';

const require = createRequire(import.meta.url);
const legacy = require('./legacy.cts');

console.log(greet('vpx'));
console.log(double(2), triple(2), data.answer, legacy.describe());
