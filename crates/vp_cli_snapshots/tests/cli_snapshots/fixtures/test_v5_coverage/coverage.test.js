import { expect, test } from 'vitest';
import { expect as bundledExpect, test as bundledTest } from 'vite-plus/test';
import { sum } from './src/covered.js';
test('covered sum', () => {
  expect(expect).toBe(bundledExpect);
  expect(test).toBe(bundledTest);
  expect(sum(20, 22)).toBe(42);
});
