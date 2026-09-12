import { expect, inject, test, vi } from 'vite-plus/test';
import { expect as upstreamExpect, vi as upstreamVi } from 'vitest';
import { matcherExpect } from 'jest-extended';

test('runner, public imports, and external matchers share assertion state', () => {
  expect(upstreamExpect).toBe(expect);
  expect(matcherExpect).toBe(expect);
  expect(upstreamVi).toBe(vi);
  expect(inject('value')).toEqual(expect.any(String));
  expect('shared').toHaveSharedState();
});
