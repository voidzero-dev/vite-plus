import { expect, test } from 'vitest';

test.sequential('works', () => {
  expect(() => { throw new Error(''); }).toThrow('');
});
