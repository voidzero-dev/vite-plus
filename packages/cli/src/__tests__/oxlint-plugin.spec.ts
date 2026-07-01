import path from 'node:path';

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

const nuxtTestFilename = path.join(
  import.meta.dirname,
  'fixtures/nuxt-test-utils/component.spec.ts',
);
const nuxtUnitTestFilename = path.join(
  import.meta.dirname,
  'fixtures/nuxt-test-utils/unit.spec.ts',
);

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
    {
      code: `import { vi } from 'vitest';\nimport { mockNuxtImport } from '@nuxt/test-utils/runtime';`,
      filename: nuxtTestFilename,
    },
    {
      code: `import { expect } from 'vitest';\nimport { startVitest } from 'vitest/node';\nimport { defineConfig } from 'vitest/config';`,
      filename: nuxtUnitTestFilename,
    },
    // Issue #2004: `vite`/`vite/*` are flagged only in config entry files, so
    // non-config files keep their `vite` imports (vite-plus is not a guaranteed
    // superset of vite's exposed surface). vitest/tsdown/@vitest are unaffected.
    {
      code: `import { defineConfig } from 'vite'`,
      filename: 'src/main.ts',
    },
    {
      code: `import { createServer } from 'vite'`,
      filename: path.join(import.meta.dirname, 'server.ts'),
    },
    {
      code: `type Api = Pick<typeof import('vite'), 'createBuilder' | 'loadConfigFromFile'>`,
      filename: 'src/deploy.ts',
    },
    {
      code: `declare module 'vite' {}`,
      filename: 'types.ts',
    },
    {
      code: `import 'vite/client'`,
      filename: 'src/env.ts',
    },
  ],
  invalid: [
    {
      code: `import { page } from '@vitest/browser/context'`,
      errors: 1,
      filename: nuxtUnitTestFilename,
      output: `import { page } from 'vite-plus/test/browser/context'`,
    },
    {
      // `vite`/`vite/*` are flagged only in config entry files (issue #2004);
      // in every other file they are preserved (see valid cases above).
      code: `import { defineConfig } from 'vite'`,
      errors: 1,
      filename: 'vite.config.ts',
      output: `import { defineConfig } from 'vite-plus'`,
    },
    {
      code: `export { defineConfig } from "vite"`,
      errors: 1,
      filename: 'vitest.config.ts',
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
      filename: 'vite.config.cts',
      output: `import foo = require('vite-plus/client')`,
    },
    {
      // In a config file both `vitest` and `vite` are flagged.
      code: `export * from 'vitest';\nimport { defineConfig } from 'vite';`,
      errors: 2,
      filename: 'vite.config.ts',
      output: `export * from 'vite-plus/test';\nimport { defineConfig } from 'vite-plus';`,
    },
    {
      code: `import { vi } from 'vitest';\nimport { startVitest } from 'vitest/node';\nimport { mockNuxtImport } from '@nuxt/test-utils/runtime';`,
      errors: 2,
      filename: path.join(import.meta.dirname, 'ordinary.spec.ts'),
      output: `import { vi } from 'vite-plus/test';\nimport { startVitest } from 'vite-plus/test/node';\nimport { mockNuxtImport } from '@nuxt/test-utils/runtime';`,
    },
    {
      code: `import { vi } from 'vitest';\nimport { mockNuxtImport } from '@nuxt/test-utils/runtime';`,
      errors: 1,
      filename: path.join(import.meta.dirname, 'ordinary.spec.ts'),
      output: `import { vi } from 'vite-plus/test';\nimport { mockNuxtImport } from '@nuxt/test-utils/runtime';`,
    },
  ],
});
