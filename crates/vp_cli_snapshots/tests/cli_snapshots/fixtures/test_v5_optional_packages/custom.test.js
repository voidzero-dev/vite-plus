import { expect, test } from 'vite-plus/test';
test('custom environment restores property descriptors without invoking getters', () => {
  expect(globalThis.__descriptorCheck).toBe(true);
});
