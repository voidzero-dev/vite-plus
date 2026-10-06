import { type OxlintConfig } from 'oxlint';
import { describe, expect, it } from 'vitest';

import { sanitizeMigratedOxlintConfig } from '../migrator/eslint.ts';

describe('Oxlint rule aliases during migration', () => {
  it('preserves supported native plugin aliases in base rules and overrides', () => {
    const config = {
      plugins: ['typescript', 'react', 'import', 'jsx-a11y', 'nextjs'],
      rules: {
        '@typescript-eslint/no-explicit-any': 'error',
        '@typescript-eslint/eslint-plugin/no-explicit-any': 'warn',
        'typescript-eslint/no-non-null-assertion': 'error',
        'eslint-plugin-react/self-closing-comp': 'error',
        'react-hooks/exhaustive-deps': 'warn',
        'eslint-plugin-react-hooks/exhaustive-deps': 'error',
        'oxlint-plugin-import/no-cycle': 'error',
        'import-x/no-cycle': 'warn',
        'jsx-a11y-x/alt-text': 'error',
        '@next/next/no-img-element': 'error',
        '@unknown/not-a-real-rule': 'warn',
      },
      overrides: [
        {
          files: ['src/**/*.ts'],
          rules: {
            '@typescript-eslint/consistent-type-imports': 'error',
          },
        },
      ],
    } as OxlintConfig;

    sanitizeMigratedOxlintConfig(config, new Set());

    expect(config.rules).toEqual({
      '@typescript-eslint/no-explicit-any': 'error',
      '@typescript-eslint/eslint-plugin/no-explicit-any': 'warn',
      'typescript-eslint/no-non-null-assertion': 'error',
      'eslint-plugin-react/self-closing-comp': 'error',
      'react-hooks/exhaustive-deps': 'warn',
      'eslint-plugin-react-hooks/exhaustive-deps': 'error',
      'oxlint-plugin-import/no-cycle': 'error',
      'import-x/no-cycle': 'warn',
      'jsx-a11y-x/alt-text': 'error',
      '@next/next/no-img-element': 'error',
    });
    expect(config.overrides?.[0].rules).toEqual({
      '@typescript-eslint/consistent-type-imports': 'error',
    });
  });

  it('preserves package-name aliases only when the JS plugin is installed', () => {
    const plugin = '@stylistic/eslint-plugin-ts';
    const config = {
      jsPlugins: [plugin],
      rules: {
        '@stylistic/eslint-plugin-ts/indent': 'error',
        '@missing/eslint-plugin-ts/indent': 'warn',
      },
    } as OxlintConfig;

    sanitizeMigratedOxlintConfig(config, new Set([plugin]));

    expect(config.rules).toEqual({ '@stylistic/eslint-plugin-ts/indent': 'error' });
  });
});
