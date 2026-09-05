import fs from 'node:fs';
import path from 'node:path';

import { type WorkspacePackage } from '../../types/index.ts';
import { hasVitestTypesInTsconfig } from '../../utils/tsconfig.ts';
import { projectUsesVitestDirectly } from '../migrator.ts';
import {
  OPT_IN_BROWSER_PROVIDERS,
  PLAYWRIGHT_PROVIDER,
  WEBDRIVERIO_PROVIDER,
  readPackageJsonIfExists,
  type DependencyBag,
} from './shared.ts';

// Workspace-wide direct-vitest signal for the SHARED sinks a monorepo root
// owns (pnpm-workspace.yaml catalog/overrides/peer rules, .yarnrc.yml catalog,
// bun catalog): `vitest` stays managed there iff ANY package in the workspace —
// the root or any sub-package — uses vitest directly. See
// `projectUsesVitestDirectly`.
export function workspaceUsesVitestDirectly(
  rootDir: string,
  packages: WorkspacePackage[] | undefined,
  preserveNuxtVitestImports = true,
): boolean {
  const rootPkg = readPackageJsonIfExists(path.join(rootDir, 'package.json')) ?? {};
  if (projectUsesVitestDirectly(rootDir, rootPkg, undefined, preserveNuxtVitestImports)) {
    return true;
  }
  if (!packages) {
    return false;
  }
  for (const pkg of packages) {
    const packageDir = path.join(rootDir, pkg.path);
    const subPkg = readPackageJsonIfExists(path.join(packageDir, 'package.json')) ?? {};
    if (projectUsesVitestDirectly(packageDir, subPkg, undefined, preserveNuxtVitestImports)) {
      return true;
    }
  }
  return false;
}

// Specifier fragments that signal vitest browser mode. Matched as substrings
// against source (see `sourceTreeReferencesAny`), so subpath imports are
// covered too. Each indicates the package drives vitest's browser runner:
//   - `@vitest/browser`         upstream, pre-migration (incl. `/context`,
//                               `/client`, … subpaths)
//   - `vite-plus/test/browser`  migrated (re-run on an already-migrated
//                               project); also covers `…/browser/context` and
//                               the `…/browser/providers/*` provider forms
//   - `vite-plus/test/{client,context,locators,matchers,utils}`  the published
//                               bare browser shims (`build.ts`
//                               `createBareBrowserShims`): each re-exports
//                               `@vitest/browser/<sub>` but DROPS the `browser`
//                               segment, so they carry no `browser` substring.
//                               The import rewriter flattens
//                               `@vitest/browser/{client,locators,matchers,
//                               utils}` to four of these in already-migrated
//                               source; `vite-plus/test/context` is reachable
//                               as the published bare export (the rewriter
//                               instead routes `@vitest/browser/context` to
//                               `vite-plus/test/browser/context`, already
//                               covered above). All five are browser-only
//                               re-exports, so they never collide with a
//                               non-browser vitest export.
//   - `vite-plus/test/plugins/browser`  prefix for the generated plugin shims
//                               (`build.ts` `PLUGIN_SHIM_ENTRIES`:
//                               `plugins/browser`, `plugins/browser-context`,
//                               `plugins/browser-client`, `plugins/browser-
//                               locators`, `plugins/browser-playwright`,
//                               `plugins/browser-preview`, `plugins/browser-
//                               webdriverio`), which re-export `@vitest/browser*`
//                               under a `/plugins/` segment that the
//                               `vite-plus/test/browser` hint does not match.
//                               One prefix covers the whole family.
//   - `vite-plus/test/internal/browser`  the published internal browser shim
//                               (`./test/internal/browser`, re-exports
//                               `vitest/internal/browser`) — also a `/browser`
//                               surface with no `vite-plus/test/browser`
//                               substring.
// Without a matching hint a package importing only one of these published
// browser surfaces (with no `@vitest/browser*` dep) would miss browser mode and
// skip pinning the direct `vitest` the browser optimizer needs resolvable from
// the package root under pnpm strict / Yarn PnP. This set is verified complete
// against every browser-surface `./test/*` export in package.json (those that
// re-export `@vitest/browser*` or `vitest/internal/browser`).
const VITEST_BROWSER_SPECIFIER_HINTS = [
  // Before v0.2, projects commonly aliased `vitest` to
  // `@voidzero-dev/vite-plus-test`, whose browser exports used these paths.
  'vitest/browser',
  'vitest/plugins/browser',
  '@vitest/browser',
  'vite-plus/test/browser',
  'vite-plus/test/plugins/browser',
  'vite-plus/test/internal/browser',
  'vite-plus/test/client',
  'vite-plus/test/context',
  'vite-plus/test/locators',
  'vite-plus/test/matchers',
  'vite-plus/test/utils',
] as const;

// Specifier fragments that signal the WEBDRIVERIO provider specifically. Each
// is a prefix, matched as a substring, so subpath imports (`/context`,
// `/provider`, …) are covered too:
//   - `vitest/browser-webdriverio`, `vitest/browser/providers/webdriverio`, and
//     `vitest/plugins/browser-webdriverio` are legacy
//     `@voidzero-dev/vite-plus-test` exports reached through the `vitest` alias
//   - `@vitest/browser-webdriverio`            pre-migration (incl. `/provider`,
//                                              `/context` subpaths)
//   - `vite-plus/test/browser-webdriverio`     migrated (re-run); covers
//                                              `…/context`
//   - `vite-plus/test/browser/providers/webdriverio`  migrated provider-subpath
//                                              form — the import rewriter maps
//                                              `@vitest/browser-webdriverio/provider`
//                                              here, so an already-migrated
//                                              project can contain it. Without
//                                              this hint a re-run would skip the
//                                              provider injection and the import
//                                              would break under pnpm strict /
//                                              Yarn PnP once the provider is no
//                                              longer a vite-plus runtime dep.
//   - `vite-plus/test/plugins/browser-webdriverio`  generated plugin shim that
//                                              re-exports `@vitest/browser-
//                                              webdriverio` wholesale; importing
//                                              it pulls in the (now opt-in)
//                                              provider, so it signals usage too.
const WEBDRIVERIO_PROVIDER_SPECIFIER_HINTS = [
  'vitest/browser-webdriverio',
  'vitest/browser/providers/webdriverio',
  'vitest/plugins/browser-webdriverio',
  '@vitest/browser-webdriverio',
  'vite-plus/test/browser-webdriverio',
  'vite-plus/test/browser/providers/webdriverio',
  'vite-plus/test/plugins/browser-webdriverio',
] as const;

// Specifier fragments that signal the PLAYWRIGHT provider specifically — the
// playwright analogue of WEBDRIVERIO_PROVIDER_SPECIFIER_HINTS (same prefix /
// substring matching for `/provider`, `/context` subpaths). Playwright is opt-in
// just like webdriverio: vite-plus no longer bundles `@vitest/browser-playwright`
// at runtime, so a source-only user (e.g. `vite.config.ts` importing the
// provider via a `vite-plus/test/browser-playwright` shim with no declared dep)
// must still have the provider kept/injected for the rewritten import to resolve.
const PLAYWRIGHT_PROVIDER_SPECIFIER_HINTS = [
  // Legacy `@voidzero-dev/vite-plus-test` exports reached through the `vitest`
  // alias. These must be detected before rewriteAllImports changes the prefix.
  'vitest/browser-playwright',
  'vitest/browser/providers/playwright',
  'vitest/plugins/browser-playwright',
  '@vitest/browser-playwright',
  'vite-plus/test/browser-playwright',
  'vite-plus/test/browser/providers/playwright',
  'vite-plus/test/plugins/browser-playwright',
] as const;

// Per-provider source-scan hint lists, used to build the `providerSourceModes`
// map passed to `rewritePackageJson`.
const BROWSER_PROVIDER_SPECIFIER_HINTS: Record<string, readonly string[]> = {
  [WEBDRIVERIO_PROVIDER]: WEBDRIVERIO_PROVIDER_SPECIFIER_HINTS,
  [PLAYWRIGHT_PROVIDER]: PLAYWRIGHT_PROVIDER_SPECIFIER_HINTS,
};

// TypeScript/JavaScript source extensions scanned for browser-mode hints.
const VITEST_SCAN_EXTENSIONS = new Set([
  '.ts',
  '.mts',
  '.cts',
  '.tsx',
  '.js',
  '.mjs',
  '.cjs',
  '.jsx',
]);

// Directories never worth scanning for browser-mode hints — generated output,
// installed deps, VCS metadata. Skipped at every recursion level.
const VITEST_SCAN_SKIP_DIRS = new Set([
  'node_modules',
  'dist',
  'build',
  'out',
  'coverage',
  '.git',
  '.next',
  '.nuxt',
  '.svelte-kit',
  '.vite',
  '.cache',
]);

/**
 * Detect whether a package uses vitest's browser mode.
 *
 * Upstream `@vitest/browser` injects `optimizeDeps.include` entries of the form
 * `vitest > expect-type` (and `vitest > @vitest/snapshot > magic-string`,
 * `vitest > @vitest/expect > chai`). Vite resolves the leading `vitest` segment
 * from the Vite config root, so `vitest` MUST be resolvable as a package from
 * the consuming package's directory. In a pnpm strict (non-hoisted) layout,
 * `vitest` pulled in only transitively via `vite-plus` is NOT reachable from the
 * package root — the optimizer then fails with `Failed to resolve dependency`
 * and the browser test page hangs forever.
 *
 * When this returns true the migration adds `vitest` as a direct
 * devDependency so it is hoisted next to the package and the optimizer chain
 * resolves. The signal is any of the package's TS/JS files (config, workspace
 * config under any name, or test file) referencing `@vitest/browser*` or
 * `vite-plus/test/browser*`. The scan recurses through the package directory
 * (skipping `node_modules`, build output, VCS metadata) so browser config in a
 * non-standard filename or browser imports in test files are all caught.
 *
 * Recursion stops at nested `package.json` boundaries: a workspace sub-package
 * is a separate package that the migration scans on its own pass, so the root
 * package must not inherit a browser-mode signal from a sub-package.
 */
function sourceTreeMatches(
  projectPath: string,
  matchesContent: (content: string) => boolean,
): boolean {
  return sourceTreeMatchesEach(projectPath, [matchesContent])[0];
}

/**
 * Evaluate several independent content predicates in a SINGLE traversal.
 *
 * Each eligible source file is read exactly once and offered to every predicate
 * that has not yet matched. Traversal stops as soon as every predicate has
 * matched, mirroring the early return of a single-predicate scan.
 *
 * Semantics per predicate are identical to running `sourceTreeMatches` for it
 * on its own — same traversal order, same skip directories, same nested-package
 * boundary, same "unreadable file is ignored" behaviour. Only the number of
 * reads changes: N predicates cost one pass instead of N.
 */
function sourceTreeMatchesEach(
  projectPath: string,
  predicates: readonly ((content: string) => boolean)[],
): boolean[] {
  const results = Array.from({ length: predicates.length }, () => false);
  let undecided = predicates.length;
  if (undecided === 0) {
    return results;
  }

  // Returns true to unwind the whole traversal once nothing is left to decide.
  const scanDir = (dir: string, isRoot: boolean): boolean => {
    let entries: fs.Dirent[];
    try {
      entries = fs.readdirSync(dir, { withFileTypes: true });
    } catch {
      return false;
    }
    // A nested package.json marks a separate workspace package — it is migrated
    // (and scanned) on its own pass, so don't let its files leak into this one.
    if (!isRoot && entries.some((e) => e.isFile() && e.name === 'package.json')) {
      return false;
    }
    for (const entry of entries) {
      const entryPath = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        if (VITEST_SCAN_SKIP_DIRS.has(entry.name)) {
          continue;
        }
        if (scanDir(entryPath, false)) {
          return true;
        }
      } else if (entry.isFile() && VITEST_SCAN_EXTENSIONS.has(path.extname(entry.name))) {
        let content: string;
        try {
          content = fs.readFileSync(entryPath, 'utf8');
        } catch {
          // Unreadable file — ignore and keep scanning.
          continue;
        }
        for (let index = 0; index < predicates.length; index++) {
          if (results[index]) {
            continue;
          }
          if (predicates[index](content)) {
            results[index] = true;
            undecided--;
          }
        }
        if (undecided === 0) {
          return true;
        }
      }
    }
    return false;
  };

  scanDir(projectPath, true);
  return results;
}

// A hint list as a reusable content predicate, so the same matcher can be run
// standalone or enrolled in a combined multi-signal traversal.
function hintMatcher(hints: readonly string[]): (content: string) => boolean {
  return (content) => hints.some((hint) => content.includes(hint));
}

function sourceTreeReferencesAny(projectPath: string, hints: readonly string[]): boolean {
  return sourceTreeMatches(projectPath, hintMatcher(hints));
}

// Source surfaces that deliberately retain the upstream `vitest` package
// identity. Extracted so the combined scan and the standalone helper share one
// definition. See `sourceTreeReferencesRetainedVitestModule`.
function matchesRetainedVitestModule(content: string): boolean {
  return (
    /\bdeclare\s+module\s+['"]vitest(?:\/[^'"]*)?['"]/.test(content) ||
    content.includes('vitest/package.json') ||
    /\brequire\.resolve\s*\(\s*['"]vitest(?:\/[^'"]*)?['"]/.test(content) ||
    /\bimport\.meta\.resolve\s*\(\s*['"]vitest(?:\/[^'"]*)?['"]/.test(content)
  );
}

function findPackageTsconfigFiles(projectPath: string): string[] {
  const files: string[] = [];
  const scanDir = (dir: string, isRoot: boolean): void => {
    let entries: fs.Dirent[];
    try {
      entries = fs.readdirSync(dir, { withFileTypes: true });
    } catch {
      return;
    }
    if (!isRoot && entries.some((entry) => entry.isFile() && entry.name === 'package.json')) {
      return;
    }
    for (const entry of entries) {
      const entryPath = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        if (!VITEST_SCAN_SKIP_DIRS.has(entry.name)) {
          scanDir(entryPath, false);
        }
      } else if (entry.isFile() && /^tsconfig(?:\.[\w-]+)?\.json$/i.test(entry.name)) {
        files.push(entryPath);
      }
    }
  };
  scanDir(projectPath, true);
  return files;
}

export function hasNuxtTestUtilsDependency(pkg: DependencyBag): boolean {
  return [pkg.dependencies, pkg.devDependencies, pkg.optionalDependencies].some(
    (dependencies) => dependencies?.['@nuxt/test-utils'] !== undefined,
  );
}

// Normal imports and triple-slash type directives from `vitest` are rewritten
// to `vite-plus/test` later in the same migration and therefore do not justify
// a lasting direct dependency. Module augmentations, `vitest/package.json`, and
// compilerOptions.types entries deliberately retain the upstream package
// identity, so keep Vitest package-local for those surfaces.
export function sourceTreeReferencesRetainedVitestModule(projectPath: string): boolean {
  return (
    findPackageTsconfigFiles(projectPath).some(hasVitestTypesInTsconfig) ||
    sourceTreeMatches(projectPath, matchesRetainedVitestModule)
  );
}

export function usesVitestBrowserMode(projectPath: string): boolean {
  return sourceTreeReferencesAny(projectPath, VITEST_BROWSER_SPECIFIER_HINTS);
}

// Source-only signal that a package targets the WEBDRIVERIO provider — used to
// allow the edgedriver/geckodriver builds even when no dep is declared yet (the
// webdriverio-specific postinstall hazard; playwright has no such drivers). See
// `usesVitestBrowserMode` for the shared traversal semantics (extensions, skip
// dirs, nested-package boundary).
export function usesWebdriverioProvider(projectPath: string): boolean {
  return sourceTreeReferencesAny(projectPath, WEBDRIVERIO_PROVIDER_SPECIFIER_HINTS);
}

// Source-scan signal per opt-in browser provider, used to inject the (opt-in,
// no-longer-bundled) provider + its framework peer even when no dep is declared
// yet (e.g. a `vite.config.ts` importing the provider via a `vite-plus/test`
// shim). Mirrors `usesWebdriverioProvider`'s scan for each provider.
export function collectProviderSourceModes(projectPath: string): Record<string, boolean> {
  const matches = sourceTreeMatchesEach(
    projectPath,
    OPT_IN_BROWSER_PROVIDERS.map((provider) =>
      hintMatcher(BROWSER_PROVIDER_SPECIFIER_HINTS[provider]),
    ),
  );
  const modes: Record<string, boolean> = {};
  OPT_IN_BROWSER_PROVIDERS.forEach((provider, index) => {
    modes[provider] = matches[index];
  });
  return modes;
}

/** All per-package source-scan signals the migration needs for one package. */
export interface PackageSourceScanSignals {
  /** @see usesVitestBrowserMode */
  browserMode: boolean;
  /** @see sourceTreeReferencesRetainedVitestModule */
  retainedModule: boolean;
  /** @see collectProviderSourceModes */
  providerSourceModes: Record<string, boolean>;
}

/**
 * Collect every per-package source signal in ONE traversal of the package tree.
 *
 * `usesVitestBrowserMode`, `sourceTreeReferencesRetainedVitestModule` and each
 * provider scan in `collectProviderSourceModes` all walk the same file set with
 * the same rules, so running them separately read every eligible source file
 * once per signal. Enrolling them as predicates in a single pass makes that one
 * read per file while leaving each individual result unchanged.
 *
 * The tsconfig `types` short-circuit of `sourceTreeReferencesRetainedVitestModule`
 * is preserved: when a tsconfig already settles the retained-module signal, its
 * source predicate is not enrolled at all, so the tree is not scanned for it.
 */
export function collectPackageSourceScanSignals(projectPath: string): PackageSourceScanSignals {
  const retainedFromTsconfig = findPackageTsconfigFiles(projectPath).some(hasVitestTypesInTsconfig);

  const predicates: ((content: string) => boolean)[] = [
    hintMatcher(VITEST_BROWSER_SPECIFIER_HINTS),
    ...OPT_IN_BROWSER_PROVIDERS.map((provider) =>
      hintMatcher(BROWSER_PROVIDER_SPECIFIER_HINTS[provider]),
    ),
  ];
  if (!retainedFromTsconfig) {
    predicates.push(matchesRetainedVitestModule);
  }

  const matches = sourceTreeMatchesEach(projectPath, predicates);

  const providerSourceModes: Record<string, boolean> = {};
  OPT_IN_BROWSER_PROVIDERS.forEach((provider, index) => {
    providerSourceModes[provider] = matches[index + 1];
  });

  return {
    browserMode: matches[0],
    retainedModule: retainedFromTsconfig || matches[1 + OPT_IN_BROWSER_PROVIDERS.length] === true,
    providerSourceModes,
  };
}
