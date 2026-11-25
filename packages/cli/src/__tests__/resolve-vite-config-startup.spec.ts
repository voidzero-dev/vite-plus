import { expect, test, vi } from 'vitest';

// `vp create` imports resolve-vite-config.ts at startup but often only looks up
// config files. Vite must load only when `resolveViteConfig` runs, through its
// dynamic import, so those paths do not pay for loading Vite.
vi.mock('vite', () => {
  throw new Error('resolve-vite-config.ts loaded vite on import');
});

test('importing resolve-vite-config does not load vite', async () => {
  const { findViteConfigUp, hasViteConfig } = await import('../resolve-vite-config.ts');
  expect(findViteConfigUp).toBeTypeOf('function');
  expect(hasViteConfig).toBeTypeOf('function');
});
