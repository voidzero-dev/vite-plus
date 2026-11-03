import { expect, test } from 'vitest';
import { sum } from './src/covered.js';
test('covered sum', () => { expect(sum(20, 22)).toBe(42); });
