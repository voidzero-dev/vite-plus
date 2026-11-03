import { expect, test } from 'vite-plus/test';

import { value } from './value.ts';

test('requires the original source value', () => {
  expect(value).toBe(1);
});
