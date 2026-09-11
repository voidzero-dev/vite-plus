import { describe, expect, it } from 'vitest';

import { extractOverrideTargetName } from './package-overrides.ts';

describe('extractOverrideTargetName', () => {
  it.each([
    ['vite-plus', 'vite-plus'],
    ['vite-plus@^0.3.0', 'vite-plus'],
    ['@vitest/coverage-v8@4', '@vitest/coverage-v8'],
    ['app>vite-plus', 'vite-plus'],
    ['app@1>test>@vitest/browser-playwright@4', '@vitest/browser-playwright'],
    ['app/@vitest/coverage-v8@4', '@vitest/coverage-v8'],
    ['**/vitest', 'vitest'],
    ['pkg@>4', 'pkg'],
    ['pkg@>=4', 'pkg'],
  ])('extracts %s', (selector, expected) => {
    expect(extractOverrideTargetName(selector)).toBe(expected);
  });
});
