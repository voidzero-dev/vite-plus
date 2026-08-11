import { describe, expect, it } from 'vitest';

import { unexpectedHooksArgsError } from '../args.js';

describe('unexpectedHooksArgsError', () => {
  it('rejects leftover positional operands', () => {
    expect(unexpectedHooksArgsError({ _: ['.custom-hooks'] })).toBe(
      'Unexpected argument ".custom-hooks". Use --hooks-dir <path> to set a custom hooks directory.',
    );
  });

  it('rejects unknown options', () => {
    expect(unexpectedHooksArgsError({ _: [], dir: '.custom-hooks' })).toBe(
      'Unknown option "--dir".',
    );
  });

  it('allows known flags', () => {
    expect(
      unexpectedHooksArgsError({ _: [], 'hooks-dir': '.custom-hooks', help: false, h: false }),
    ).toBeNull();
  });
});
