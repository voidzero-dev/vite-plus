import { expect, it, vi } from 'vite-plus/test';

import { value } from './dep.ts';

// `vi.mock` is written AFTER the import on purpose. The static mocker must
// recognize `vi` imported from `vite-plus/test` and hoist this call above the
// import, otherwise the real `./dep` loads first and `value` stays 'real'.
// Upstream vitest >=4.1.9 recognizes the `vite-plus/test` redistribution
// natively (no vite-plus-side patch/shim required).
vi.mock('./dep.ts', () => ({ value: 'mocked' }));

it('hoists vi.mock() above imports for the vite-plus/test specifier', () => {
  expect(value).toBe('mocked');
});
