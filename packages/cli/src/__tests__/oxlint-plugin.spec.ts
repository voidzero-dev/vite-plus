import { RuleTester } from 'oxlint/plugins-dev';
import { describe, expect, it } from 'vitest';

import {
  createDefaultVitePlusLintConfig,
  ensureVitePlusImportRuleDefaults,
  PREFER_VITE_PLUS_IMPORTS_RULE,
  PREFER_VITE_PLUS_IMPORTS_RULE_NAME,
  VITE_PLUS_OXLINT_PLUGIN_SPECIFIER,
} from '../oxlint-plugin-config.js';
import { preferVitePlusImportsRule, rewriteVitePlusImportSpecifier } from '../oxlint-plugin.js';

describe('oxlint plugin config defaults', () => {
  it('adds vite-plus js plugin and lint rule defaults', () => {
    expect(
      createDefaultVitePlusLintConfig({
        includeTypeAwareDefaults: true,
      }),
    ).toEqual({
      jsPlugins: [
        {
          name: 'vite-plus',
          specifier: VITE_PLUS_OXLINT_PLUGIN_SPECIFIER,
        },
      ],
      options: {
        typeAware: true,
        typeCheck: true,
      },
      rules: {
        [PREFER_VITE_PLUS_IMPORTS_RULE]: 'error',
      },
    });
  });

  it('preserves explicit user settings while backfilling defaults', () => {
    expect(
      ensureVitePlusImportRuleDefaults({
        jsPlugins: [VITE_PLUS_OXLINT_PLUGIN_SPECIFIER],
        rules: {
          [PREFER_VITE_PLUS_IMPORTS_RULE]: 'off',
          eqeqeq: 'warn',
        },
      }),
    ).toEqual({
      jsPlugins: [VITE_PLUS_OXLINT_PLUGIN_SPECIFIER],
      rules: {
        [PREFER_VITE_PLUS_IMPORTS_RULE]: 'off',
        eqeqeq: 'warn',
      },
    });
  });
});

describe('rewriteVitePlusImportSpecifier', () => {
  it('rewrites supported vite and vitest specifiers', () => {
    expect(rewriteVitePlusImportSpecifier('vite')).toBe('vite-plus');
    expect(rewriteVitePlusImportSpecifier('vite/client')).toBe('vite-plus/client');
    expect(rewriteVitePlusImportSpecifier('vitest')).toBe('vite-plus/test');
    expect(rewriteVitePlusImportSpecifier('vitest/config')).toBe('vite-plus');
    expect(rewriteVitePlusImportSpecifier('@vitest/browser')).toBe('vite-plus/test/browser');
    expect(rewriteVitePlusImportSpecifier('@vitest/browser/context')).toBe(
      'vite-plus/test/browser/context',
    );
    expect(rewriteVitePlusImportSpecifier('@vitest/browser/client')).toBe('vite-plus/test/client');
    expect(rewriteVitePlusImportSpecifier('@vitest/browser/locators')).toBe(
      'vite-plus/test/locators',
    );
    expect(rewriteVitePlusImportSpecifier('@vitest/browser/matchers')).toBe(
      'vite-plus/test/matchers',
    );
    expect(rewriteVitePlusImportSpecifier('@vitest/browser/utils')).toBe('vite-plus/test/utils');
    expect(rewriteVitePlusImportSpecifier('@vitest/browser-playwright/context')).toBe(
      'vite-plus/test/browser/context',
    );
    expect(rewriteVitePlusImportSpecifier('@vitest/browser-playwright/provider')).toBe(
      'vite-plus/test/browser/providers/playwright',
    );
    expect(rewriteVitePlusImportSpecifier('@vitest/browser-preview/provider')).toBe(
      'vite-plus/test/browser/providers/preview',
    );
    expect(rewriteVitePlusImportSpecifier('@vitest/browser-webdriverio/provider')).toBe(
      'vite-plus/test/browser/providers/webdriverio',
    );
    expect(rewriteVitePlusImportSpecifier('@vitest/browser-playwright/locators')).toBeNull();
    // `vitest/package.json` must NOT be rewritten — `vite-plus` does not export
    // `./test/package.json`, so rewriting would break resolution. Mirrors the
    // migrate rewriter's exclusion.
    expect(rewriteVitePlusImportSpecifier('vitest/package.json')).toBeNull();
    // ...but other `vitest/<sub>` specifiers still rewrite normally.
    expect(rewriteVitePlusImportSpecifier('vitest/node')).toBe('vite-plus/test/node');
    expect(rewriteVitePlusImportSpecifier('tsx')).toBeNull();
  });
});

new RuleTester({
  languageOptions: {
    sourceType: 'module',
  },
}).run(PREFER_VITE_PLUS_IMPORTS_RULE_NAME, preferVitePlusImportsRule, {
  valid: [
    `import { defineConfig } from 'vite-plus'`,
    `export { expect } from 'vite-plus/test'`,
    // `vitest/package.json` must NOT be autofixed — `vite-plus` has no
    // `./test/package.json` export, so a rewrite would break resolution.
    `import pkg from 'vitest/package.json'`,
    {
      code: `declare module 'vite-plus/test/browser' {}`,
      filename: 'types.ts',
    },
    {
      code: `type BrowserClient = typeof import('vite-plus/test/client')`,
      filename: 'types.ts',
    },
    {
      code: `type PlaywrightProvider = typeof import('vite-plus/test/browser/providers/playwright')`,
      filename: 'types.ts',
    },
    {
      code: `type TestFn = typeof import('vite-plus/test')['test']`,
      filename: 'types.ts',
    },
    // `declare module 'vitest…'` / `declare module '@vitest/browser…'` are
    // intentionally NOT autofixed — they target the upstream module identity
    // so type augmentations merge with what `vite-plus/test*` re-exports.
    {
      code: `declare module 'vitest' {}`,
      filename: 'types.ts',
    },
    {
      code: `declare module 'vitest/node' {}`,
      filename: 'types.ts',
    },
    {
      code: `declare module '@vitest/browser' {}`,
      filename: 'types.ts',
    },
    {
      code: `declare module '@vitest/browser/context' {}`,
      filename: 'types.ts',
    },
    {
      code: `declare module '@vitest/browser-playwright' {}`,
      filename: 'types.ts',
    },
    {
      code: `declare module '@vitest/browser-playwright/context' {}`,
      filename: 'types.ts',
    },
  ],
  invalid: [
    {
      // `declare module 'vite'` IS rewritten — the vite family doesn't
      // re-export upstream vite types so augmentation works against either id.
      code: `declare module 'vite' {}`,
      errors: 1,
      filename: 'types.ts',
      output: `declare module 'vite-plus' {}`,
    },
    {
      code: `import { defineConfig } from 'vite'`,
      errors: 1,
      output: `import { defineConfig } from 'vite-plus'`,
    },
    {
      code: `export { defineConfig } from "vite"`,
      errors: 1,
      output: `export { defineConfig } from "vite-plus"`,
    },
    {
      code: `const mod = import('vitest/config')`,
      errors: 1,
      output: `const mod = import('vite-plus')`,
    },
    {
      code: `type TestFn = typeof import('vitest')['test']`,
      errors: 1,
      filename: 'types.ts',
      output: `type TestFn = typeof import('vite-plus/test')['test']`,
    },
    {
      code: `type BrowserClient = typeof import('@vitest/browser/client')`,
      errors: 1,
      filename: 'types.ts',
      output: `type BrowserClient = typeof import('vite-plus/test/client')`,
    },
    {
      code: `import { expect } from '@vitest/browser/matchers'`,
      errors: 1,
      output: `import { expect } from 'vite-plus/test/matchers'`,
    },
    {
      code: `import { getElementError } from '@vitest/browser/utils'`,
      errors: 1,
      output: `import { getElementError } from 'vite-plus/test/utils'`,
    },
    {
      code: `type PlaywrightProvider = typeof import('@vitest/browser-playwright/provider')`,
      errors: 1,
      filename: 'types.ts',
      output: `type PlaywrightProvider = typeof import('vite-plus/test/browser/providers/playwright')`,
    },
    {
      code: `import foo = require('vite/client')`,
      errors: 1,
      filename: 'types.ts',
      output: `import foo = require('vite-plus/client')`,
    },
    {
      code: `export * from 'vitest';\nimport { defineConfig } from 'vite';`,
      errors: 2,
      output: `export * from 'vite-plus/test';\nimport { defineConfig } from 'vite-plus';`,
    },
  ],
});
