import { expect, test } from 'vitest';

// `vite-plus/pack` re-exports the tsdown build bundled into core, so it must keep tsdown's runtime API.
test('exposes every tsdown value export from vite-plus/pack', async () => {
  const [pack, tsdown] = await Promise.all([import('../pack.ts'), import('tsdown')]);
  const expected = Object.keys(tsdown).filter((key) => key !== 'default');
  expect(expected.length, 'sanity: tsdown should expose value exports').toBeGreaterThan(0);
  const available = new Map(Object.entries(pack));
  const missing = expected.filter((key) => available.get(key) === undefined);
  expect(missing, 'tsdown value exports missing from vite-plus/pack').toEqual([]);
});
