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

// Keep augmentations on the upstream module whose types the shims re-export.
function isOxlintFamilyDeclareModuleSpecifier(specifier: string): boolean {
  return (
    specifier === OXLINT_PACKAGE ||
    specifier.startsWith(`${OXLINT_PACKAGE}/`) ||
    specifier === OXLINT_PLUGINS_PACKAGE
  );
}

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

// Names outside this config surface use the legacy plugin API. Keep this list
// in sync with the Oxlint rules in crates/vp_migration/src/import_rewriter.rs.
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

  // These entry points expose only plugin APIs. Bare `oxlint` also exposes
  // config APIs, so it needs the named-binding checks below.
  if (specifier === OXLINT_PLUGINS_PACKAGE) {
    return VITE_PLUS_LINT_PLUGINS;
  }

  if (specifier === OXLINT_PLUGINS_DEV_SUBPATH) {
    return VITE_PLUS_LINT_PLUGINS_DEV;
  }

  return null;
}

function moduleBindingName(node: ESTree.ImportSpecifier['imported']): string {
  return node.type === 'Identifier' ? node.name : node.value;
}

// Replacing the source affects the whole import. Require named plugin bindings
// only: the shim has neither config exports nor a default export.
function importsOxlintPluginApi(node: ESTree.ImportDeclaration): boolean {
  return (
    node.specifiers.length > 0 &&
    node.specifiers.every(
      (specifier) =>
        specifier.type === 'ImportSpecifier' &&
        !OXLINT_CONFIG_SURFACE_EXPORTS.has(moduleBindingName(specifier.imported)),
    )
  );
}

function quoteSpecifier(literal: ESTree.StringLiteral, replacement: string): string {
  const quote = literal.raw?.startsWith("'") ? "'" : '"';
  return `${quote}${replacement}${quote}`;
}

type PackageDependencies = {
  dependencies?: Record<string, string>;
  devDependencies?: Record<string, string>;
  optionalDependencies?: Record<string, string>;
  peerDependencies?: Record<string, string>;
};

type PackageMatchCache = Map<string, { mtimeMs: number; matches: boolean }>;

// Separate decisions share the same mtime-based invalidation in editor sessions.
const nuxtTestUtilsPackageCache: PackageMatchCache = new Map();
const oxlintOwnerPackageCache: PackageMatchCache = new Map();

function isUpstreamVitestSpecifier(specifier: string): boolean {
  return specifier === 'vitest' || specifier.startsWith('vitest/');
}

function nearestPackageMatches(
  filename: string,
  cache: PackageMatchCache,
  matchesPackage: (pkg: PackageDependencies) => boolean,
): boolean {
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
        // Bypass the cache when stat fails; a sentinel could reuse stale data.
      }
      const cached = mtimeMs === undefined ? undefined : cache.get(packageJsonPath);
      if (cached !== undefined && cached.mtimeMs === mtimeMs) {
        return cached.matches;
      }
      let matches = false;
      try {
        const pkg = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8')) as PackageDependencies;
        matches = matchesPackage(pkg);
      } catch {
        // Invalid or unreadable package metadata cannot opt into the exception.
      }
      if (mtimeMs !== undefined) {
        cache.set(packageJsonPath, { mtimeMs, matches });
      }
      return matches;
    }
    const parent = path.dirname(directory);
    if (parent === directory) {
      return false;
    }
    directory = parent;
  }
}

function nearestPackageUsesNuxtTestUtils(filename: string): boolean {
  return nearestPackageMatches(filename, nuxtTestUtilsPackageCache, (pkg) =>
    [pkg.dependencies, pkg.devDependencies, pkg.optionalDependencies].some(
      (dependencies) => dependencies?.['@nuxt/test-utils'] !== undefined,
    ),
  );
}

// Match the migrator's published-plugin exemption. Development-only APIs do not
// exempt a package; optional @oxlint/plugins is a consumer runtime dependency.
function nearestPackageOwnsOxlintApi(filename: string): boolean {
  return nearestPackageMatches(
    filename,
    oxlintOwnerPackageCache,
    (pkg) =>
      pkg.optionalDependencies?.[OXLINT_PLUGINS_PACKAGE] !== undefined ||
      [pkg.dependencies, pkg.peerDependencies].some(
        (dependencies) =>
          dependencies?.[OXLINT_PACKAGE] !== undefined ||
          dependencies?.[OXLINT_PLUGINS_PACKAGE] !== undefined,
      ),
  );
}

function reportSpecifier(
  context: Context,
  literal: ESTree.StringLiteral,
  replacement: string,
): void {
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

function isOxlintApiSpecifier(specifier: string): boolean {
  return specifier === OXLINT_PLUGINS_PACKAGE || specifier === OXLINT_PLUGINS_DEV_SUBPATH;
}

interface ImportRewriteOptions {
  preserveUpstreamVitest: boolean;
  fileIsViteConfig: boolean;
  ownsOxlintApi: boolean;
}

function maybeReportLiteral(
  context: Context,
  literal: ESTree.Expression | ESTree.TSModuleDeclaration['id'] | null | undefined,
  { preserveUpstreamVitest, fileIsViteConfig, ownsOxlintApi }: ImportRewriteOptions,
): void {
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
  if (ownsOxlintApi && isOxlintApiSpecifier(literal.value)) {
    return;
  }

  reportSpecifier(context, literal, replacement);
}

// Bare `oxlint` needs named-binding checks beyond maybeReportLiteral's mapping.
function reportLegacyOxlintPluginApiExport(
  context: Context,
  node: ESTree.ExportNamedDeclaration,
  ownsOxlintApi: boolean,
): void {
  const literal = node.source;
  if (!literal || literal.value !== OXLINT_PACKAGE || ownsOxlintApi) {
    return;
  }
  if (node.specifiers.length === 0) {
    return;
  }
  const allPluginApi = node.specifiers.every(
    (specifier) => !OXLINT_CONFIG_SURFACE_EXPORTS.has(moduleBindingName(specifier.local)),
  );
  if (!allPluginApi) {
    return;
  }
  reportSpecifier(context, literal, VITE_PLUS_LINT_PLUGINS);
}

function reportLegacyOxlintPluginApiImport(
  context: Context,
  node: ESTree.ImportDeclaration,
  ownsOxlintApi: boolean,
): void {
  const literal = node.source;
  if (literal.value !== OXLINT_PACKAGE || ownsOxlintApi || !importsOxlintPluginApi(node)) {
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
    const options: ImportRewriteOptions = {
      preserveUpstreamVitest: false,
      fileIsViteConfig: false,
      ownsOxlintApi: false,
    };
    return {
      Program() {
        options.preserveUpstreamVitest = nearestPackageUsesNuxtTestUtils(context.filename);
        options.fileIsViteConfig = isViteConfigFile(context.filename);
        options.ownsOxlintApi = nearestPackageOwnsOxlintApi(context.filename);
      },
      ImportDeclaration(node) {
        maybeReportLiteral(context, node.source, options);
        reportLegacyOxlintPluginApiImport(context, node, options.ownsOxlintApi);
      },
      ExportAllDeclaration(node) {
        maybeReportLiteral(context, node.source, options);
      },
      ExportNamedDeclaration(node) {
        maybeReportLiteral(context, node.source, options);
        reportLegacyOxlintPluginApiExport(context, node, options.ownsOxlintApi);
      },
      ImportExpression(node) {
        maybeReportLiteral(context, node.source, options);
      },
      TSImportType(node) {
        maybeReportLiteral(context, node.source, options);
      },
      TSExternalModuleReference(node) {
        // Keep import-equals declarations unchanged, matching the migrator's
        // treatment of require calls.
        if (
          node.expression.type === 'Literal' &&
          typeof node.expression.value === 'string' &&
          isOxlintApiSpecifier(node.expression.value)
        ) {
          return;
        }
        maybeReportLiteral(context, node.expression, options);
      },
      TSModuleDeclaration(node) {
        if (node.global) {
          return;
        }
        const id = node.id;
        if (
          id?.type === 'Literal' &&
          typeof id.value === 'string' &&
          (isVitestFamilyDeclareModuleSpecifier(id.value) ||
            isOxlintFamilyDeclareModuleSpecifier(id.value))
        ) {
          return;
        }
        maybeReportLiteral(context, id, options);
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
