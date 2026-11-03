import { expect, test } from 'vite-plus/test';

test('DOM assignment affects happy-dom matchMedia', () => {
  globalThis.innerWidth = 412;
  expect(matchMedia('(max-width: 500px)').matches).toBe(true);
  globalThis.innerWidth = 812;
  expect(matchMedia('(max-width: 500px)').matches).toBe(false);
});
