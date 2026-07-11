import fs from 'node:fs';
import path from 'node:path';

import { definePlugin, defineRule } from '@oxlint/plugins';
import type { Context, ESTree } from '@oxlint/plugins';

import {
  PREFER_VITE_PLUS_IMPORTS_RULE_NAME,
  VITE_PLUS_OXLINT_PLUGIN_NAME,
} from './oxlint-plugin-config.ts';
import { VITE_CONFIG_ENTRY_BASENAMES } from './utils/config-files.ts';

// `declare module 'vitest…'` and `declare module '@vitest/browser…'` are
// intentionally preserved by `vp migrate` (see migration's import_rewriter and
// docs/guide/migrate.md) — `vite-plus/test*` is a thin re-export of upstream
// `vitest*`, so type augmentations have to target the upstream module identity
// to merge correctly. Autofixing those module declarations here would split the
// augmentation away from what imports actually resolve through.
function isVitestFamilyDeclareModuleSpecifier(specifier: string): boolean {
  return (
    specifier === 'vitest' ||
    specifier.startsWith('vitest/') ||
    specifier === '@vitest/browser' ||
    specifier.startsWith('@vitest/browser/') ||
    specifier.startsWith('@vitest/browser-')
  );
}

// Issue #2004: `vp migrate` rewrites `vite`/`vite/*` imports only in config entry
// files, so this lint rule (the parallel enforcement of the same rewrite) does
// the same. Every other file keeps its `vite` imports, since vite-plus is not a
// guaranteed superset of vite's exposed surface. The basename whitelist is the
// single source shared with the migrate rewriter, which embeds the same
// `vite-config-entry-basenames.json` at compile time (import_rewriter.rs). The
// lint rule sees one file at a time, so it recognizes the standard basenames only
// (no migrate-resolved custom path). vitest/tsdown/@vitest are unaffected.
function isViteSpecifier(specifier: string): boolean {
  return specifier === 'vite' || specifier.startsWith('vite/');
}

function isViteConfigFile(filename: string): boolean {
  return VITE_CONFIG_ENTRY_BASENAMES.has(path.basename(filename));
}

function rewriteVitePlusImportSpecifier(specifier: string): string | null {
  if (specifier === 'vite') {
    return 'vite-plus';
  }

  if (specifier.startsWith('vite/')) {
    return `vite-plus/${specifier.slice('vite/'.length)}`;
  }

  if (specifier === 'vitest/config') {
    return 'vite-plus';
  }

  if (specifier === 'vitest') {
    return 'vite-plus/test';
  }

  // `vitest/package.json` is a metadata-access pattern (reading the vitest
  // version) and `vite-plus`'s generated exports map deliberately omits
  // `./test/package.json` (see `syncTestPackageExports()` in build.ts, which
  // skips upstream's `./package.json`). Rewriting it would yield
  // `vite-plus/test/package.json`, which fails with ERR_PACKAGE_PATH_NOT_EXPORTED.
  // The original specifier still resolves through the installed `vitest`. This
  // mirrors the migrate rewriter's exclusion in import_rewriter.rs.
  if (specifier === 'vitest/package.json') {
    return null;
  }

  if (specifier.startsWith('vitest/')) {
    return `vite-plus/test/${specifier.slice('vitest/'.length)}`;
  }

  if (specifier === '@vitest/browser') {
    return 'vite-plus/test/browser';
  }

  // `@vitest/browser/context` keeps the nested path (vite-plus exports
  // `./test/browser/context`); the remaining subpaths are exposed only at the
  // bare `./test/<name>` surface, so the `/browser/` segment is dropped.
  const browserSubpathRewrites: Record<string, string> = {
    '@vitest/browser/context': 'vite-plus/test/browser/context',
    '@vitest/browser/client': 'vite-plus/test/client',
    '@vitest/browser/locators': 'vite-plus/test/locators',
    '@vitest/browser/matchers': 'vite-plus/test/matchers',
    '@vitest/browser/utils': 'vite-plus/test/utils',
  };
  if (specifier in browserSubpathRewrites) {
    return browserSubpathRewrites[specifier];
  }

  for (const [prefix, provider] of [
    ['@vitest/browser-playwright', 'playwright'],
    ['@vitest/browser-preview', 'preview'],
    ['@vitest/browser-webdriverio', 'webdriverio'],
  ] as const) {
    if (specifier === prefix) {
      return `vite-plus/test/${prefix.slice('@vitest/'.length)}`;
    }

    if (specifier === `${prefix}/context`) {
      return 'vite-plus/test/browser/context';
    }

    if (specifier === `${prefix}/provider`) {
      return `vite-plus/test/browser/providers/${provider}`;
    }
  }

  return null;
}

function quoteSpecifier(literal: ESTree.StringLiteral, replacement: string): string {
  const quote = literal.raw?.startsWith("'") ? "'" : '"';
  return `${quote}${replacement}${quote}`;
}

// Keyed by package.json path and invalidated by its mtime so a long-lived lint
// process (editor/LSP session) re-reads the manifest after the user adds or
// removes `@nuxt/test-utils`, instead of reusing the pre-edit decision forever.
const nuxtTestUtilsPackageCache = new Map<
  string,
  { mtimeMs: number; usesNuxtTestUtils: boolean }
>();

function isUpstreamVitestSpecifier(specifier: string): boolean {
  return specifier === 'vitest' || specifier.startsWith('vitest/');
}

function nearestPackageUsesNuxtTestUtils(filename: string): boolean {
  if (!path.isAbsolute(filename)) {
    return false;
  }
  let directory = path.dirname(filename);
  while (true) {
    const packageJsonPath = path.join(directory, 'package.json');
    if (fs.existsSync(packageJsonPath)) {
      let mtimeMs: number | undefined;
      try {
        mtimeMs = fs.statSync(packageJsonPath).mtimeMs;
      } catch {
        // Unreadable manifest: bypass the cache entirely below. A sentinel
        // value would collide with an entry cached during an earlier failure
        // and pin the pre-edit decision.
      }
      const cached =
        mtimeMs === undefined ? undefined : nuxtTestUtilsPackageCache.get(packageJsonPath);
      if (cached !== undefined && cached.mtimeMs === mtimeMs) {
        return cached.usesNuxtTestUtils;
      }
      let usesNuxtTestUtils = false;
      try {
        const pkg = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8')) as {
          dependencies?: Record<string, string>;
          devDependencies?: Record<string, string>;
          optionalDependencies?: Record<string, string>;
        };
        usesNuxtTestUtils = [pkg.dependencies, pkg.devDependencies, pkg.optionalDependencies].some(
          (dependencies) => dependencies?.['@nuxt/test-utils'] !== undefined,
        );
      } catch {
        // Invalid or unreadable package metadata cannot opt into the exception.
      }
      if (mtimeMs !== undefined) {
        nuxtTestUtilsPackageCache.set(packageJsonPath, { mtimeMs, usesNuxtTestUtils });
      }
      return usesNuxtTestUtils;
    }
    const parent = path.dirname(directory);
    if (parent === directory) {
      return false;
    }
    directory = parent;
  }
}

function maybeReportLiteral(
  context: Context,
  literal: ESTree.Expression | ESTree.TSModuleDeclaration['id'] | null | undefined,
  preserveUpstreamVitest = false,
  fileIsViteConfig = false,
) {
  if (!literal || literal.type !== 'Literal' || typeof literal.value !== 'string') {
    return;
  }
  if (preserveUpstreamVitest && isUpstreamVitestSpecifier(literal.value)) {
    return;
  }
  // Issue #2004: keep `vite`/`vite/*` imports outside config entry files.
  if (!fileIsViteConfig && isViteSpecifier(literal.value)) {
    return;
  }

  const replacement = rewriteVitePlusImportSpecifier(literal.value);
  if (!replacement) {
    return;
  }

  context.report({
    node: literal,
    messageId: 'preferVitePlusImports',
    data: {
      from: literal.value,
      to: replacement,
    },
    fix(fixer) {
      return fixer.replaceText(literal, quoteSpecifier(literal, replacement));
    },
  });
}

export const preferVitePlusImportsRule = defineRule({
  meta: {
    type: 'problem',
    docs: {
      description: 'Prefer vite-plus module specifiers over vite and vitest packages.',
      recommended: true,
      url: 'https://github.com/voidzero-dev/vite-plus/issues/1301',
    },
    fixable: 'code',
    messages: {
      preferVitePlusImports: "Use '{{to}}' instead of '{{from}}' in Vite+ projects.",
    },
  },
  createOnce(context: Context) {
    let preserveUpstreamVitest = false;
    let fileIsViteConfig = false;
    return {
      Program() {
        preserveUpstreamVitest = nearestPackageUsesNuxtTestUtils(context.filename);
        fileIsViteConfig = isViteConfigFile(context.filename);
      },
      ImportDeclaration(node) {
        maybeReportLiteral(context, node.source, preserveUpstreamVitest, fileIsViteConfig);
      },
      ExportAllDeclaration(node) {
        maybeReportLiteral(context, node.source, preserveUpstreamVitest, fileIsViteConfig);
      },
      ExportNamedDeclaration(node) {
        maybeReportLiteral(context, node.source, preserveUpstreamVitest, fileIsViteConfig);
      },
      ImportExpression(node) {
        maybeReportLiteral(context, node.source, preserveUpstreamVitest, fileIsViteConfig);
      },
      TSImportType(node) {
        maybeReportLiteral(context, node.source, preserveUpstreamVitest, fileIsViteConfig);
      },
      TSExternalModuleReference(node) {
        maybeReportLiteral(context, node.expression, preserveUpstreamVitest, fileIsViteConfig);
      },
      TSModuleDeclaration(node) {
        if (node.global) {
          return;
        }
        const id = node.id;
        if (
          id?.type === 'Literal' &&
          typeof id.value === 'string' &&
          isVitestFamilyDeclareModuleSpecifier(id.value)
        ) {
          return;
        }
        maybeReportLiteral(context, id, preserveUpstreamVitest, fileIsViteConfig);
      },
    };
  },
});

const plugin = definePlugin({
  meta: {
    name: VITE_PLUS_OXLINT_PLUGIN_NAME,
  },
  rules: {
    [PREFER_VITE_PLUS_IMPORTS_RULE_NAME]: preferVitePlusImportsRule,
  },
});

export default plugin;
export { rewriteVitePlusImportSpecifier };
