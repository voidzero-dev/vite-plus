import { expect } from 'vitest';
export const matcherExpect = expect;
expect.extend({
  toHaveSharedState(actual) {
    return { pass: actual === 'shared', message: () => 'expected shared assertion state' };
  },
});
