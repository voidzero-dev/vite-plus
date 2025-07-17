import * as console from '@log/index.ts';
import { script1 } from '@scripts/script1.ts';
import { expect, test } from 'vitest';

test('console', () => {
  expect(console.log).toBeDefined();
});

test('script', () => {
  expect(script1()).toBe('script 1');
});
