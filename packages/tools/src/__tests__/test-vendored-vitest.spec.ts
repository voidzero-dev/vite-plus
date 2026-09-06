import { describe, expect, test } from 'vitest';

import { adaptBrandedViteAssertions } from '../test-vendored-vitest.ts';

describe('vendored Vitest bridge', () => {
  test('checks the deliberate branding changes and retains unrelated assertions', () => {
    const source = [
      ...Array.from(
        { length: 8 },
        () =>
          'expect(messages).toMatchInlineSnapshot(`"VITE_CONFIG_NATIVE_IGNORE_WARNING: \\`native\\`"`)',
      ),
      'expect(messages).toMatchInlineSnapshot(`"unrelated warning"`)',
      'expect(config).toMatchInlineSnapshot(`{ "jsonValue": "vite" }`)',
    ].join('\n');
    const result = adaptBrandedViteAssertions(source);
    expect(result.match(/expect\(messages\)\.toEqual\(\[\]\)/g)).toHaveLength(8);
    expect(result).toContain('"jsonValue": "@voidzero-dev/vite-plus-core"');
    expect(result).toContain('expect(messages).toMatchInlineSnapshot(`"unrelated warning"`)');
  });

  test('requires a new review if the upstream tests change', () => {
    expect(() => adaptBrandedViteAssertions('expect(messages).toEqual([])')).toThrow(
      'Review the vendored Vite branding assertions',
    );
  });
});
