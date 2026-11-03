import { expect, test } from 'vite-plus/test';

import { value } from './value.ts';

test('reads the source value', () => {
  expect(value).toBeGreaterThan(0);
});
