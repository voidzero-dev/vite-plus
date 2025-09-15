import { expect, test } from 'vitest';
import { myFunction } from '../src';

test('myFunction', () => {
  expect(myFunction()).toBe('Hello, world!');
});
