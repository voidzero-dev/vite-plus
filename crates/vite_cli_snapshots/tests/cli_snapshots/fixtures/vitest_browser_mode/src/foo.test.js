import { describe, expect, it } from 'vitest';

import foo from './foo';

describe('foo', () => {
  it('should equal "foo"', () => {
    expect(foo).toBe('foo');
  });
});
