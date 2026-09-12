import { expect, test, vi } from 'vite-plus/test';
import { expect as rawExpect, vi as rawVi } from 'vitest';

expect.extend({ toBeAnswer(received) { return { pass: received === 42, message: () => 'Expected 42' }; } });
test('installed runner shares custom matchers and mock state', () => {
  expect(rawExpect).toBe(expect);
  expect(rawVi).toBe(vi);
  rawExpect(42).toBeAnswer();
  const mock = vi.fn();
  mock();
  rawExpect(mock).toHaveBeenCalledOnce();
});
