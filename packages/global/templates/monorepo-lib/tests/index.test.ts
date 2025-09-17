import { expect, test } from 'vitest';
import { myFunction } from '../src/index.ts';

test('myFunction', () => {
  expect(myFunction()).toBe('Hello, world!');
});
