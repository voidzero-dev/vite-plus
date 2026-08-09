import fs from 'node:fs';
import path from 'node:path';

import { definePlugin, defineRule } from '@oxlint/plugins';
import type { Context, ESTree } from '@oxlint/plugins';

import {
  PREFER_VITE_PLUS_IMPORTS_RULE_NAME,
  VITE_PLUS_OXLINT_PLUGIN_NAME,
} from './oxlint-plugin-config.ts';
import viteConfigEntryBasenames from './vite-config-entry-basenames.json' with { type: 'json' };

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
const VITE_CONFIG_FILE_BASENAMES = new Set(viteConfigEntryBasenames);

function isViteSpecifier(specifier: string): boolean {
  return specifier === 'vite' || specifier.startsWith('vite/');
}

function isViteConfigFile(filename: string): boolean {
  return VITE_CONFIG_FILE_BASENAMES.has(path.basename(filename));
}

const OXLINT_PACKAGE = 'oxlint';
const OXLINT_PLUGINS_PACKAGE = '@oxlint/plugins';
const OXLINT_PLUGINS_DEV_SUBPATH = 'oxlint/plugins-dev';
const VITE_PLUS_LINT_PLUGINS = 'vite-plus/lint/plugins';
const VITE_PLUS_LINT_PLUGINS_DEV = 'vite-plus/lint/plugins-dev';

// Everything the `oxlint` package still exports from its main entry: the config
// surface. Those imports are correct as they are, so the rule must not redirect
// them. Any other name in an `import ... from 'oxlint'` belongs to the
// pre-`@oxlint/plugins` authoring API, such as `defineRule`, `Context`, or
// `ESTree`. That API no longer resolves once the migration strips the
// standalone `oxlint` dependency.
//
// This is a denylist, not an allowlist of about 60 plugin type names. The
// denylist is small and stable, and an unrecognized name falls on the side of
// fixing the breakage. It mirrors the `rewrite-oxlint-plugin-api-import` rule
// in `crates/vp_migration/src/import_rewriter.rs`. The two MUST stay in sync.
const OXLINT_CONFIG_SURFACE_EXPORTS = new Set([
  'defineConfig',
  'AllowWarnDeny',
  'DummyRule',
  'DummyRuleMap',
  'ExternalPluginEntry',
  'ExternalPluginsConfig',
  'OxlintConfig',
  'OxlintEnv',
  'OxlintGlobals',
  'OxlintOverride',
  'RuleCategories',
]);

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

  // The Oxlint JS-plugin authoring API. Vite+ bundles Oxlint, so a project's
  // own plugin should reach the API through `vite-plus`. Otherwise it pins
  // `@oxlint/plugins` against whatever Oxlint the bundled linter runs. These
  // two specifiers serve nothing but the plugin API, so they always rewrite.
  // `reportLegacyOxlintPluginApiImport` handles the ambiguous bare `oxlint`
  // specifier.
  if (specifier === OXLINT_PLUGINS_PACKAGE) {
    return VITE_PLUS_LINT_PLUGINS;
  }

  if (specifier === OXLINT_PLUGINS_DEV_SUBPATH) {
    return VITE_PLUS_LINT_PLUGINS_DEV;
  }

  return null;
}

function importedName(specifier: ESTree.ImportSpecifier): string | undefined {
  const imported = specifier.imported;
  if (imported.type === 'Identifier') {
    return imported.name;
  }
  return typeof imported.value === 'string' ? imported.value : undefined;
}

/**
 * True when an `import ... from 'oxlint'` names at least one binding outside
 * Oxlint's config surface. Such an import reaches for the plugin authoring API.
 *
 * Default, namespace, and bare side-effect imports name no binding. They
 * return `false`, so the rule leaves them alone instead of risking a wrong
 * rewrite.
 */
function importsOxlintPluginApi(node: ESTree.ImportDeclaration): boolean {
  return node.specifiers.some(
    (specifier) =>
      specifier.type === 'ImportSpecifier' &&
      !OXLINT_CONFIG_SURFACE_EXPORTS.has(importedName(specifier) ?? ''),
  );
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

function reportSpecifier(context: Context, literal: ESTree.StringLiteral, replacement: string) {
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

  reportSpecifier(context, literal, replacement);
}

/**
 * `import { defineRule } from 'oxlint'` → `'vite-plus/lint/plugins'`.
 *
 * This is separate from {@link maybeReportLiteral} because the specifier string
 * alone cannot decide the bare `oxlint` case. That specifier still serves the
 * config surface. Only an `ImportDeclaration` shows the named bindings that
 * tell the two surfaces apart. Re-export, `require`, and dynamic `import`
 * statements therefore do not get this rewrite.
 */
function reportLegacyOxlintPluginApiImport(context: Context, node: ESTree.ImportDeclaration) {
  const literal = node.source;
  if (literal.value !== OXLINT_PACKAGE || !importsOxlintPluginApi(node)) {
    return;
  }
  reportSpecifier(context, literal, VITE_PLUS_LINT_PLUGINS);
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
        reportLegacyOxlintPluginApiImport(context, node);
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
