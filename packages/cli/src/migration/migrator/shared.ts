import fs from 'node:fs';
import path from 'node:path';

import * as prompts from '@voidzero-dev/vite-plus-prompts';
import semver from 'semver';

import { VITEST_VERSION, VITE_PLUS_OVERRIDE_PACKAGES } from '../../utils/constants.ts';
import { readJsonFile } from '../../utils/json.ts';
import { detectPackageMetadata } from '../../utils/package.ts';
import { displayRelative } from '../../utils/path.ts';
import { addManualStep, addMigrationWarning, type MigrationReport } from '../report.ts';

// All known lint-staged config file names.
// JSON-parseable ones come first so rewriteLintStagedConfigFile can rewrite them.
export const LINT_STAGED_JSON_CONFIG_FILES = ['.lintstagedrc.json', '.lintstagedrc'] as const;

export const LINT_STAGED_OTHER_CONFIG_FILES = [
  '.lintstagedrc.yaml',
  '.lintstagedrc.yml',
  '.lintstagedrc.mjs',
  'lint-staged.config.mjs',
  '.lintstagedrc.cjs',
  'lint-staged.config.cjs',
  '.lintstagedrc.js',
  'lint-staged.config.js',
  '.lintstagedrc.ts',
  'lint-staged.config.ts',
  '.lintstagedrc.mts',
  'lint-staged.config.mts',
  '.lintstagedrc.cts',
  'lint-staged.config.cts',
] as const;

export const LINT_STAGED_ALL_CONFIG_FILES = [
  ...LINT_STAGED_JSON_CONFIG_FILES,
  ...LINT_STAGED_OTHER_CONFIG_FILES,
] as const;

// packages that are replaced with vite-plus
export const REMOVE_PACKAGES = [
  'oxlint',
  'oxlint-tsgolint',
  'oxfmt',
  'tsdown',
  '@vitest/browser',
  '@vitest/browser-preview',
] as const;

// The opt-in browser providers. Unlike `@vitest/browser`/preview these are NOT
// bundled by vite-plus or stripped from users (so they stay out of
// REMOVE_PACKAGES); each drags a heavy non-optional framework peer
// (`playwright` / `webdriverio`) that non-browser consumers must not be forced
// to install. The migration keeps a provider the user actually targets in their
// own deps, pinned to the bundled vitest version.
export const WEBDRIVERIO_PROVIDER = '@vitest/browser-webdriverio';

export const PLAYWRIGHT_PROVIDER = '@vitest/browser-playwright';

// All opt-in browser providers handled identically by the migration: kept in
// the user's deps (pinned to the bundled vitest), framework peer ensured, stale
// forcing pins dropped, while their catalog entries are PRESERVED.
export const OPT_IN_BROWSER_PROVIDERS = [WEBDRIVERIO_PROVIDER, PLAYWRIGHT_PROVIDER] as const;

// Provider names whose stale pnpm overrides / resolutions are dropped during
// migration: everything vite-plus owns (REMOVE_PACKAGES) plus the user-owned
// opt-in providers. The provider DEP is preserved, but a leftover
// override/resolution pin to another version would WIN over the direct dep and
// misalign the provider against the bundled vitest — so the stale forcing pin is
// dropped while the dependency itself stays installed. NOTE: catalog deletion
// uses REMOVE_PACKAGES (not this set) on purpose — a catalog entry is only a
// version *definition*, and deleting it could dangle a surviving `catalog:`
// reference (e.g. in peerDependencies) and break install.
export const PROVIDER_OVERRIDE_DROP_NAMES = [
  ...REMOVE_PACKAGES,
  ...OPT_IN_BROWSER_PROVIDERS,
] as const;

// When a browser provider package is removed, its runtime peer dependency
// must be preserved in devDependencies so browser tests continue to work.
export const BROWSER_PROVIDER_PEER_DEPS: Record<string, string> = {
  '@vitest/browser-playwright': 'playwright',
  '@vitest/browser-webdriverio': 'webdriverio',
};

// Lockstep sibling packages whose declared version a browser provider's runtime
// framework peer should reuse (they publish together). Keyed by the peer name.
export const PROVIDER_PEER_VERSION_SIBLINGS: Record<string, readonly string[]> = {
  playwright: ['@playwright/test'],
  webdriverio: ['@wdio/cli', '@wdio/globals'],
};

// A package's declared spec across all four dependency fields, or undefined.
export function findDeclaredSpec(pkg: DependencyBag, name: string): string | undefined {
  return (
    pkg.dependencies?.[name] ??
    pkg.devDependencies?.[name] ??
    pkg.peerDependencies?.[name] ??
    pkg.optionalDependencies?.[name]
  );
}

// A deterministic spec for a browser provider's framework peer instead of `*`:
// reference the catalog when it already owns the peer, otherwise reuse a declared
// lockstep sibling's version (concrete, or a catalog reference resolved to its
// concrete value), falling back to `*` only when there is no sibling. See
// npmx.dev #27.
export function resolveProviderPeerSpec(
  pkg: DependencyBag,
  peer: string,
  supportCatalog: boolean,
  catalogDependencyResolver?: CatalogDependencyResolver,
): string {
  if (supportCatalog && catalogDependencyResolver?.('catalog:', peer) !== undefined) {
    return 'catalog:';
  }
  for (const sibling of PROVIDER_PEER_VERSION_SIBLINGS[peer] ?? []) {
    const spec = findDeclaredSpec(pkg, sibling);
    const resolved = spec?.startsWith('catalog:')
      ? catalogDependencyResolver?.(spec, sibling)
      : spec;
    // Only reuse a concrete version: a `catalog:` entry may itself alias another
    // protocol, and npm:/workspace:/file: specs aren't versions to copy.
    if (resolved && !resolved.includes(':')) {
      return resolved;
    }
  }
  return '*';
}

// Browser-provider package names that, when present in the user's deps
// before migration, signal vitest browser mode even if no source file
// imports them. This covers config-only browser-mode setups (e.g.
// `test.browser.provider: 'playwright'` in `vite.config.ts`) where the
// provider package is declared in `devDependencies` but never `import`ed.
export const VITEST_BROWSER_DEP_NAMES = [
  '@vitest/browser',
  '@vitest/browser-preview',
  '@vitest/browser-playwright',
  '@vitest/browser-webdriverio',
] as const;

// Common case (`!usesVitest`): vite-plus consumes upstream vitest itself, so a
// lingering `vitest` entry — a managed pin, a stale `npm:@voidzero-dev/vite-plus-test@*`
// wrapper alias, or a `catalog:` reference — must be REMOVED from every sink so
// it arrives transitively through vite-plus and a future `vp update vite-plus`
// keeps it correct with no pin to drift. The `@vitest/*` family is left
// untouched (those are direct-usage signals handled elsewhere).
//
// The removal only applies when `vitest` is a key vite-plus actually manages in
// the active override config. When a caller-supplied `VP_OVERRIDE_PACKAGES`
// omits the `vitest` key (see the migration-vitest-unmanaged-override snap
// test), `vitest` is NOT managed, so a `vitest` entry there is the user's own
// and must be left untouched.
export const VITEST_IS_MANAGED_OVERRIDE = 'vitest' in VITE_PLUS_OVERRIDE_PACKAGES;

// Fallback specs used when normalizing a stale wrapper alias. Real user
// ranges (e.g. `vitest: ^3.0.0`) are preserved — only the wrapper alias is
// rewritten. For `vitest`, we substitute the vitest version vite-plus
// bundles so any `catalog:` reference the user still has resolves cleanly.
export const LEGACY_WRAPPER_FALLBACK_VERSIONS: Record<string, string> = {
  vitest: VITEST_VERSION,
};

export type PackageJsonDependencyField =
  | 'devDependencies'
  | 'dependencies'
  | 'peerDependencies'
  | 'optionalDependencies';

export type CatalogDependencyResolver = ((
  catalogSpec: string,
  dependencyName: string,
) => string | undefined) & {
  preferredCatalogSpec: string;
};

export function warnMigration(message: string, report?: MigrationReport) {
  addMigrationWarning(report, message);
  if (!report) {
    prompts.log.warn(message);
  }
}

export function infoMigration(message: string, report?: MigrationReport) {
  addManualStep(report, message);
  if (!report) {
    prompts.log.info(message);
  }
}

export function checkViteVersion(projectPath: string): boolean {
  return checkPackageVersion(projectPath, 'vite', '7.0.0');
}

export function checkVitestVersion(projectPath: string): boolean {
  return checkPackageVersion(projectPath, 'vitest', '4.0.0');
}

/**
 * Check the package version is supported by auto migration
 * @param projectPath - The path to the project
 * @param name - The name of the package
 * @param minVersion - The minimum version of the package
 * @returns true if the package version is supported by auto migration
 */
function checkPackageVersion(projectPath: string, name: string, minVersion: string): boolean {
  const metadata = detectPackageMetadata(projectPath, name);
  if (!metadata || metadata.name !== name) {
    return true;
  }
  if (semver.satisfies(metadata.version, `<${minVersion}`)) {
    const packageJsonFilePath = path.join(projectPath, 'package.json');
    prompts.log.error(
      `✘ ${name}@${metadata.version} in ${displayRelative(packageJsonFilePath)} is not supported by auto migration`,
    );
    prompts.log.info(`Please upgrade ${name} to version >=${minVersion} first`);
    return false;
  }
  return true;
}

type PnpmPeerDependencyRules = {
  allowAny?: string[];
  allowedVersions?: Record<string, string>;
  [key: string]: unknown;
};

export type PnpmPackageJsonSettings = {
  overrides?: Record<string, string>;
  peerDependencyRules?: PnpmPeerDependencyRules;
  allowBuilds?: Record<string, boolean>;
  onlyBuiltDependencies?: string[];
  [key: string]: unknown;
};

export function isPlainRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

export type DependencyBag = {
  dependencies?: Record<string, string>;
  devDependencies?: Record<string, string>;
  optionalDependencies?: Record<string, string>;
  peerDependencies?: Record<string, string>;
};

export function readPackageJsonIfExists(packageJsonPath: string): DependencyBag | undefined {
  if (!fs.existsSync(packageJsonPath)) {
    return undefined;
  }
  try {
    return readJsonFile(packageJsonPath);
  } catch {
    return undefined;
  }
}

// pnpm v10 introduced the map-shaped `allowBuilds` and removed the implicit
// "build everything" default; v9 (>= 9.5) gates builds via the list-shaped
// `onlyBuiltDependencies`. Both live in pnpm-workspace.yaml or in
// `package.json`'s `pnpm` field — vp migrate writes to whichever sink the
// rest of the migration is already touching.
export function pnpmMajor(version: string | undefined): number | undefined {
  const coerced = version ? semver.coerce(version)?.version : undefined;
  return coerced ? semver.major(coerced) : undefined;
}
