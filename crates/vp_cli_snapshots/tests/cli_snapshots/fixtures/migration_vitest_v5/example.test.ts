import { expect, test } from 'vitest';

test.sequential('compatibility', () => {
  expect(Promise.resolve(42)).resolves.toBe(42);
  expect(() => { throw new Error(''); }).toThrow('');
});
