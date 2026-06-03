import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { RuleTester } from 'oxlint/plugins-dev';
import { describe, expect, it } from 'vitest';

import {
  createDefaultVitePlusLintConfig,
  ensureVitePlusImportRuleDefaults,
  PREFER_VITE_PLUS_IMPORTS_RULE,
  PREFER_VITE_PLUS_IMPORTS_RULE_NAME,
  REQUIRE_PNPM_VITE_ALIAS_RULE,
  REQUIRE_PNPM_VITE_ALIAS_RULE_NAME,
  VITE_PLUS_OXLINT_PLUGIN_SPECIFIER,
} from '../oxlint-plugin-config.js';
import {
  pnpmWorkspaceAliasesViteToVitePlusCore,
  preferVitePlusImportsRule,
  requirePnpmViteAliasRule,
  rewriteVitePlusImportSpecifier,
} from '../oxlint-plugin.js';

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
        [REQUIRE_PNPM_VITE_ALIAS_RULE]: 'error',
      },
    });
  });

  it('preserves explicit user settings while backfilling defaults', () => {
    expect(
      ensureVitePlusImportRuleDefaults({
        jsPlugins: [VITE_PLUS_OXLINT_PLUGIN_SPECIFIER],
        rules: {
          [PREFER_VITE_PLUS_IMPORTS_RULE]: 'off',
          [REQUIRE_PNPM_VITE_ALIAS_RULE]: 'warn',
          eqeqeq: 'warn',
        },
      }),
    ).toEqual({
      jsPlugins: [VITE_PLUS_OXLINT_PLUGIN_SPECIFIER],
      rules: {
        [PREFER_VITE_PLUS_IMPORTS_RULE]: 'off',
        [REQUIRE_PNPM_VITE_ALIAS_RULE]: 'warn',
        eqeqeq: 'warn',
      },
    });
  });
});

describe('pnpmWorkspaceAliasesViteToVitePlusCore', () => {
  it('detects pnpm workspace overrides that redirect vite through a catalog alias', () => {
    expect(
      pnpmWorkspaceAliasesViteToVitePlusCore(`
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@latest
overrides:
  vite: "catalog:"
`),
    ).toBe(true);
  });

  it('ignores workspaces without a Vite+ vite override', () => {
    expect(
      pnpmWorkspaceAliasesViteToVitePlusCore(`
catalog:
  vite: ^7.0.0
overrides:
  react: catalog:
`),
    ).toBe(false);
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
  ],
  invalid: [
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
      code: `declare module '@vitest/browser-playwright' {}`,
      errors: 1,
      filename: 'types.ts',
      output: `declare module 'vite-plus/test/browser-playwright' {}`,
    },
    {
      code: `declare module '@vitest/browser-playwright/context' {}`,
      errors: 1,
      filename: 'types.ts',
      output: `declare module 'vite-plus/test/browser/context' {}`,
    },
    {
      code: `type BrowserClient = typeof import('@vitest/browser/client')`,
      errors: 1,
      filename: 'types.ts',
      output: `type BrowserClient = typeof import('vite-plus/test/client')`,
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

function createPnpmWorkspacePackage(options: { hasViteDependency: boolean; packageDir?: string }) {
  const workspaceDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-oxlint-pnpm-alias-'));
  const packageDir = path.join(workspaceDir, options.packageDir ?? 'apps/website');
  const configFile = path.join(packageDir, 'vite.config.ts');

  fs.mkdirSync(packageDir, { recursive: true });
  fs.writeFileSync(
    configFile,
    `import { defineConfig } from 'vite-plus';\n\nexport default defineConfig({});\n`,
  );
  fs.writeFileSync(
    path.join(workspaceDir, 'pnpm-workspace.yaml'),
    [
      'packages:',
      '  - apps/*',
      'catalog:',
      '  vite: npm:@voidzero-dev/vite-plus-core@latest',
      '  vite-plus: latest',
      'overrides:',
      '  vite: "catalog:"',
      '',
    ].join('\n'),
  );
  fs.writeFileSync(
    path.join(packageDir, 'package.json'),
    JSON.stringify(
      {
        name: 'website',
        scripts: {
          dev: 'vp dev',
          build: 'tsc && vp build',
          preview: 'vp preview',
        },
        devDependencies: {
          ...(options.hasViteDependency ? { vite: 'catalog:' } : {}),
          'vite-plus': 'catalog:',
        },
      },
      null,
      2,
    ),
  );
  return { configFile, workspaceDir };
}

const validPnpmApp = createPnpmWorkspacePackage({ hasViteDependency: true });
const invalidPnpmApp = createPnpmWorkspacePackage({ hasViteDependency: false });
const validPnpmLibrary = createPnpmWorkspacePackage({
  hasViteDependency: false,
  packageDir: 'packages/utils',
});
fs.writeFileSync(
  path.join(validPnpmLibrary.workspaceDir, 'packages/utils/package.json'),
  JSON.stringify(
    {
      name: 'utils',
      scripts: {
        dev: 'vp pack --watch',
        build: 'vp pack',
      },
      devDependencies: {
        'vite-plus': 'catalog:',
      },
    },
    null,
    2,
  ),
);

new RuleTester({
  languageOptions: {
    sourceType: 'module',
  },
}).run(REQUIRE_PNPM_VITE_ALIAS_RULE_NAME, requirePnpmViteAliasRule, {
  valid: [
    {
      code: `import { defineConfig } from 'vite-plus';\n\nexport default defineConfig({});`,
      filename: validPnpmApp.configFile,
    },
    {
      code: `import { defineConfig } from 'vite-plus';\n\nexport default defineConfig({});`,
      filename: validPnpmLibrary.configFile,
    },
    {
      code: `export const app = true;`,
      filename: path.join(path.dirname(invalidPnpmApp.configFile), 'src/main.ts'),
    },
  ],
  invalid: [
    {
      code: `import { defineConfig } from 'vite-plus';\n\nexport default defineConfig({});`,
      errors: [{ messageId: 'requirePnpmViteAlias' }],
      filename: invalidPnpmApp.configFile,
    },
  ],
});
