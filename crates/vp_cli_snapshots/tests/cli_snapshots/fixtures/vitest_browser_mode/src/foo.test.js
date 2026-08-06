import { describe, expect, it } from 'vite-plus/test';

import foo from './foo';

describe('foo', () => {
  it('should equal "foo"', () => {
    expect(foo).toBe('foo');
  });
});
