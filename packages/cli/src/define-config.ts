import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';

import type { PluginOption, UserConfig } from '@voidzero-dev/vite-plus-core';
import type { OxfmtConfig } from 'oxfmt';
import type { OxlintConfig } from 'oxlint';
import {
  defineConfig as viteDefineConfig,
  defineProject as viteDefineProject,
  type ConfigEnv,
  type TestProjectConfiguration,
  type UserProjectConfigExport,
  type UserProjectConfigFn,
  type UserWorkspaceConfig,
} from 'vitest/config';
import type { InlineConfig as VitestInlineConfig } from 'vitest/node';

import type { CreateTemplateEntry } from './create/org-manifest.ts';
import type { PackUserConfig } from './pack.ts';
import type { RunConfig } from './run-config.ts';
import type { StagedConfig } from './staged-config.ts';
import { CONFIG_METADATA_ENV, VITEST_VERSION } from './utils/constants.ts';

declare module '@voidzero-dev/vite-plus-core' {
  interface UserConfig {
    /**
     * Options for oxlint
     */
    lint?: OxlintConfig;

    fmt?: OxfmtConfig;

    /**
     * Defaults for the `vp check` composite command. Each flag mirrors the
     * matching CLI option (`--no-fmt` / `--no-lint`) and only affects
     * `vp check`; standalone `vp fmt` / `vp lint` are unaffected.
     */
    check?: {
      /**
       * Run the format step in `vp check`.
       * @default true
       */
      fmt?: boolean;

      /**
       * Run the lint step in `vp check`. Type-check still runs when both
       * `lint.options.typeAware` and `lint.options.typeCheck` are enabled.
       * @default true
       */
      lint?: boolean;
    };

    pack?: PackUserConfig | PackUserConfig[];

    run?: RunConfig;

    staged?: StagedConfig;

    /**
     * Options for `vp create`.
     *
     * See `rfcs/create-org-default-templates.md` for the full specification.
     */
    create?: {
      /**
       * When `vp create` is invoked with no template argument, use this
       * value as if the user had typed it — typically a scope like
       * `'@your-org'` paired with a `@your-org/create` package that exposes a
       * `createConfig.templates` manifest. Can also name a local
       * `create.templates` entry.
       */
      defaultTemplate?: string;

      /**
       * Local templates available to `vp create` inside this monorepo. Each
       * entry is shown in the `vp create` picker by `name`/`description`; its
       * `template` resolves like any specifier (a workspace package name, a
       * relative `./path`, a `vite:*` builtin, a GitHub URL, or an npm package).
       */
      templates?: CreateTemplateEntry[];
    };

    /**
     * Vitest test configuration.
     *
     * Vitest augments vite's `UserConfig` with a `test` field via
     * `declare module 'vite'`, but vite-plus-core is a fork of vite so that
     * augmentation does not apply here. Re-declare it locally so user
     * configs like `defineConfig({ test: { globals: true } })` typecheck.
     */
    test?: VitestInlineConfig;
  }
}

type ViteUserConfigFnObject = (env: ConfigEnv) => UserConfig;
type ViteUserConfigFnPromise = (env: ConfigEnv) => Promise<UserConfig>;
type ViteUserConfigFn = (env: ConfigEnv) => UserConfig | Promise<UserConfig>;
type ViteUserConfigExport =
  | UserConfig
  | Promise<UserConfig>
  | ViteUserConfigFnObject
  | ViteUserConfigFnPromise
  | ViteUserConfigFn;

/**
 * `require` anchored at THIS module's location so `require.resolve` reaches
 * the `vitest` / `@vitest/*` family that the `vite-plus` package directly
 * depends on — even from a consumer project where they are only transitive.
 * Used to locate the bundled `vitest` package (its `package.json`), NOT to
 * resolve module ENTRIES: `require.resolve` applies the `require` export
 * condition, which selects Vitest's CJS entries — for the bare `vitest` root
 * a throw-stub (`index.cjs` — "Vitest cannot be imported … using require()"),
 * and for subpaths the CJS build (e.g. `vitest/config` → `config.cjs`) rather
 * than the ESM entry the test server's module graph needs. Module entries are
 * resolved through Vite's own resolver instead (see
 * [[vitePlusVitestResolverPlugin]]), which honours ESM conditions.
 *
 * `define-config.ts` is bundled by tsdown in BOTH formats: ESM (`shims: true`,
 * which defines a module-scoped `__dirname`) and CJS (where `__dirname` is the
 * Node global). The guard picks `__dirname` whenever it exists and otherwise
 * falls back to `import.meta.url`; tsdown rewrites the latter to
 * `pathToFileURL(__filename).href` in the CJS bundle, so it is safe in both.
 */
const vitePlusRequire = createRequire(
  typeof __dirname !== 'undefined' ? __dirname : import.meta.url,
);

/**
 * Absolute path to THIS module, used as a `this.resolve` importer so Vite's
 * resolver roots the `vitest` / `@vitest/*` family at `vite-plus`'s own
 * location — reaching its direct deps (`vitest`, `vitest/*`, `@vitest/browser*`)
 * even from a consumer project where they are only transitive.
 *
 * `import.meta.url` is native in the ESM bundle and rewritten by tsdown to
 * `pathToFileURL(__filename).href` in the CJS bundle, so it is a valid file URL
 * in both.
 */
const vitePlusModuleFile = fileURLToPath(import.meta.url);

/**
 * Absolute path to the bundled `vitest` package's `package.json`, used as a
 * second `this.resolve` importer. The nested `@vitest/*` family (`@vitest/expect`,
 * `@vitest/runner`, `@vitest/snapshot`, …) are dependencies of `vitest` itself —
 * not direct deps of `vite-plus` — so under pnpm's isolated layout they are
 * reachable from `vitest`'s location but not from [[vitePlusModuleFile]].
 * Resolving `package.json` is condition-agnostic, so this is safe with
 * `require.resolve`. Cached; `null` once an attempt has failed so we never retry.
 */
let vitestAnchor: string | null | undefined;
function getVitestAnchor(): string | null {
  if (vitestAnchor !== undefined) {
    return vitestAnchor;
  }
  try {
    vitestAnchor = vitePlusRequire.resolve('vitest/package.json');
  } catch {
    vitestAnchor = null;
  }
  return vitestAnchor;
}

/**
 * Match the `vitest` / `@vitest/*` family of bare specifiers — the imports a
 * browser-mode Vite dev server must resolve. Any query string is stripped
 * first; relative (`./`), absolute (`/`), and virtual (`\0`) ids never match.
 *
 * Exported for unit testing.
 */
export function isVitestFamilySpecifier(id: string): boolean {
  const bare = id.split('?')[0];
  if (bare.startsWith('.') || bare.startsWith('/') || bare.startsWith('\0')) {
    return false;
  }
  return (
    bare === 'vitest' ||
    bare.startsWith('vitest/') ||
    bare === '@vitest/browser' ||
    bare.startsWith('@vitest/')
  );
}

/**
 * Rescue `vitest` / `@vitest/*` resolution for browser-mode tests.
 *
 * In an established project that depends only on `vite-plus`, both `vitest`
 * and `@vitest/browser` are transitive deps. pnpm's isolated layout only
 * exposes a package's *direct* deps, so the browser-mode Vite dev server
 * (rooted at the consumer project) cannot resolve `vitest/internal/browser`,
 * `@vitest/expect`, etc. Non-browser tests are unaffected — vitest's own
 * module runner handles resolution there.
 *
 * This plugin re-resolves the `vitest` / `@vitest/*` family through Vite's OWN
 * resolver, but ROOTED at `vite-plus`'s location ([[vitePlusModuleFile]]) and
 * then the bundled `vitest`'s location ([[getVitestAnchor]]) BEFORE the
 * project. So every such import binds to the same physical (pinned) Vitest that
 * `vp test` spawns as the runner (see `resolveBundled` in `resolve-test.ts`)
 * and that the `vite-plus/test*` shims re-export. Were a project-local Vitest
 * preferred instead, a project that keeps its own `vitest` dependency would
 * split the run across two physical Vitest module instances — the runner
 * (bundled) vs. the test files' `vi`/`expect`/runner internals (project) — a
 * classic source of internal-state and mock-hoisting mismatches. For the common
 * migrated layout (a project depending only on `vite-plus`) nothing in this
 * family is resolvable from the project root under pnpm's isolated layout
 * anyway, so default resolution would return `null` there regardless;
 * bundle-first only changes the project-keeps-its-own-`vitest` case, which is
 * exactly the case we want pinned.
 *
 * Resolution goes through `this.resolve` (NOT [[vitePlusRequire]].resolve) so
 * Vite's ESM export conditions are honoured: a raw `require.resolve` would pick
 * Vitest's CJS `require`-condition entry — a throw-stub for the bare `vitest`
 * root (`index.cjs`), and the CJS build for subpaths (e.g. `vitest/config` →
 * `config.cjs`) instead of the ESM entry. Two bundled anchors are tried because `@vitest/browser*` are
 * direct deps of `vite-plus` (reachable from [[vitePlusModuleFile]]) while the
 * nested `@vitest/*` family are deps of `vitest` (reachable only from the
 * `vitest` anchor). The project root remains the last resort for any family id
 * the bundled tree cannot resolve, so this is never worse than deferring first.
 *
 * Two intentional limits of routing through `this.resolve`:
 *   - An EXPLICIT project `resolve.alias` / `resolve.dedupe` on the vitest
 *     family takes precedence (Vite's pipeline applies it even from a bundled
 *     anchor). Neither is set by default in Vitest 4.x, so this only affects
 *     projects that deliberately re-point the family — treated as an opt-out of
 *     pinning, not defeated silently.
 *   - Coverage providers (`@vitest/coverage-v8` / `-istanbul`) are NOT shipped
 *     with `vite-plus`, so they hit the project fallback below. Under
 *     `--coverage`, a project-installed provider of a different version pairs
 *     with the bundled runner; Vitest only WARNS on the version skew and then
 *     runs mixed versions (its provider `_initialize` logs and continues, it
 *     does not throw), which silently yields unreliable coverage — so
 *     [[vitePlusCoverageVersionGuardPlugin]] fails fast on a mismatch instead.
 */
function vitePlusVitestResolverPlugin(): PluginOption {
  return {
    name: 'vite-plus:vitest-resolver',
    enforce: 'pre',
    async resolveId(id, importer, options) {
      if (!isVitestFamilySpecifier(id)) {
        return null;
      }
      // The redirected imports are all clean bare specifiers; a queried id is
      // outside the scope of this resolver — let the default resolver handle it.
      if (id.includes('?')) {
        return null;
      }
      // Re-resolve from vite-plus's own (pinned) tree via Vite's resolver so the
      // runner and every imported Vitest module are a single physical instance.
      // `skipSelf` avoids infinite recursion back into this plugin.
      const vitestAnchorPath = getVitestAnchor();
      const bundledAnchors = vitestAnchorPath
        ? [vitePlusModuleFile, vitestAnchorPath]
        : [vitePlusModuleFile];
      for (const anchor of bundledAnchors) {
        const resolved = await this.resolve(id, anchor, { ...options, skipSelf: true });
        if (resolved) {
          return resolved;
        }
      }
      // Last resort: default project-rooted resolution for any family id the
      // bundled tree cannot resolve (e.g. coverage providers not shipped with
      // vite-plus).
      return this.resolve(id, importer, { ...options, skipSelf: true });
    },
  };
}

/**
 * Packages that register Vitest `expect` matchers via `expect.extend()` from
 * a side-effect import. When Vite serves these from a separate module graph
 * than the test runtime, the matchers register on a different `expect`
 * instance and `expect(...).<matcher>` is undefined at call time (vitest
 * issue #897). Inlining them into the test server's module graph forces
 * registration on the same instance.
 *
 * Only packages that are **installed** in the consumer project are inlined.
 * Absent packages are silently skipped so the server-deps optimizer never
 * tries to resolve a name that does not exist in the project's node_modules.
 *
 * The check is deferred to a `configResolved` plugin hook so that
 * `resolvedConfig.root` points at the actual project root (the value vite has
 * already normalised), rather than relying on `process.cwd()` at config-load
 * time (which can differ in workspace / monorepo setups).
 *
 * Exported for unit testing.
 */
export const AUTO_INLINE_DEPS: ReadonlyArray<string> = [
  '@testing-library/jest-dom',
  '@storybook/test',
  'jest-extended',
];

/**
 * Compute the merged `test.server.deps.inline` list for a given project root,
 * appending only those entries from [[AUTO_INLINE_DEPS]] that are actually
 * installed in the project.
 *
 * Returns `null` when nothing needs to change (either `inline: true` or an
 * empty result), so the caller can skip the mutation step.
 *
 * Exported for unit testing. The `_createRequire` parameter lets tests inject
 * a controlled resolver without needing to spy on Node's ESM module namespace.
 */
export function computeAutoInlineList(
  existingInline: (string | RegExp)[] | true | undefined,
  projectRoot: string,
  _createRequire: (from: string) => { resolve: (id: string) => string } = createRequire,
): (string | RegExp)[] | null {
  // User opted into "inline everything" — don't touch.
  if (existingInline === true) {
    return null;
  }
  // Build a require resolver anchored at the project root so we only
  // inline packages that are actually installed there.
  const projectRequire = _createRequire(`${projectRoot}/package.json`);
  // Start from a copy of the user-supplied array (or a fresh array when
  // none was provided) so the originating user-config object is not mutated.
  const merged: (string | RegExp)[] = Array.isArray(existingInline) ? [...existingInline] : [];
  for (const pkg of AUTO_INLINE_DEPS) {
    // Skip if already covered by a string or regexp entry.
    if (merged.some((entry) => entry === pkg || (entry instanceof RegExp && entry.test(pkg)))) {
      continue;
    }
    try {
      projectRequire.resolve(pkg);
    } catch {
      // Package not installed in the project — skip silently.
      continue;
    }
    merged.push(pkg);
  }
  // Return null when there's nothing new to inline so callers can bail early.
  const hadEntries = Array.isArray(existingInline) ? existingInline.length : 0;
  if (merged.length === hadEntries) {
    return null;
  }
  return merged;
}

function vitePlusAutoInlineMatcherPlugin(): PluginOption {
  return {
    name: 'vite-plus:auto-inline-matcher-deps',
    enforce: 'pre',
    configResolved(resolvedConfig) {
      // Access the vitest test config via the augmented field. Vitest augments
      // vite's `UserConfig` but not `ResolvedConfig`, so we use `any` here.
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const testConfig = (resolvedConfig as any).test as
        | { server?: { deps?: { inline?: (string | RegExp)[] | true } } }
        | undefined;
      const merged = computeAutoInlineList(testConfig?.server?.deps?.inline, resolvedConfig.root);
      if (merged === null) {
        return;
      }
      // Mutate the resolved config so the finalised inline list is visible
      // to vitest when it reads test.server.deps.inline.
      if (!testConfig) {
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        (resolvedConfig as any).test = { server: { deps: { inline: merged } } };
      } else {
        if (!testConfig.server) {
          testConfig.server = {};
        }
        if (!testConfig.server.deps) {
          testConfig.server.deps = {};
        }
        testConfig.server.deps.inline = merged;
      }
    },
  };
}

/** Coverage providers vite-plus can version-check against the bundled runner. */
const KNOWN_COVERAGE_PROVIDERS: ReadonlySet<string> = new Set(['v8', 'istanbul']);

/**
 * Resolve the coverage provider package name that should be version-checked, or
 * `null` when no check applies (coverage off, or a `custom`/unknown provider
 * vite-plus does not bundle a runner for).
 *
 * Takes Vitest's OWN resolved coverage options (`enabled`/`provider`), which the
 * `configureVitest` hook exposes AFTER Vitest's CLI parser has run — so the
 * `--coverage` family of flags is already folded into `enabled`/`provider` and
 * we never re-parse `process.argv` ourselves. Unset `provider` defaults to `v8`
 * (Vitest's default).
 *
 * Exported for unit testing.
 */
export function resolveCoverageProviderToCheck(
  coverage: { enabled?: boolean; provider?: string } | undefined,
): string | null {
  if (!coverage?.enabled) {
    return null;
  }
  const name = coverage.provider ?? 'v8';
  return KNOWN_COVERAGE_PROVIDERS.has(name) ? `@vitest/coverage-${name}` : null;
}

/**
 * vite-plus bundles `vitest@VITEST_VERSION` as the test runner, but coverage
 * providers (`@vitest/coverage-v8` / `-istanbul`) are project-installed peer
 * deps it does not ship. Vitest only PRINTS A WARNING on a provider/runner
 * version skew and then runs mixed versions (verified in 4.1.9: the provider's
 * `_initialize` calls `logger.warn`, it never throws), silently producing
 * unreliable coverage. Fail fast instead.
 *
 * Exported for unit testing.
 */
export function assertCoverageProviderVersionMatch(
  providerPackageName: string,
  installedVersion: string | null | undefined,
  expectedVersion: string = VITEST_VERSION,
): void {
  if (installedVersion && installedVersion !== expectedVersion) {
    throw new Error(
      `vite-plus bundles vitest@${expectedVersion}, but ${providerPackageName}@${installedVersion} ` +
        `is installed. A coverage provider must match the test runner version: Vitest only prints a ` +
        `warning on a mismatch and then runs mixed versions, which produces unreliable coverage. ` +
        `Pin ${providerPackageName} to ${expectedVersion} in your dependencies.`,
    );
  }
}

/**
 * The bundled vitest's `package.json` path — the SAME anchor Vitest's own
 * `import('@vitest/coverage-*')` resolves against (its dist walks up from here).
 * Used as a fallback resolution anchor for the coverage guard. Lazily computed
 * and cached; `null` when the bundled vitest is somehow unreachable, in which
 * case the guard simply relies on the project-root anchor.
 */
let bundledVitestAnchorCache: string | null | undefined;
function bundledVitestAnchor(): string | null {
  if (bundledVitestAnchorCache === undefined) {
    try {
      bundledVitestAnchorCache = createRequire(import.meta.url).resolve('vitest/package.json');
    } catch {
      bundledVitestAnchorCache = null;
    }
  }
  return bundledVitestAnchorCache;
}

/**
 * Read a project-installed coverage provider's version, mirroring how Vitest
 * itself resolves the provider. Vitest install-checks it from BOTH the runner
 * root AND its own bundled dir (`isPackageExists(dep, {paths:[root, vitestDir]})`)
 * and then loads it via a bare `import('@vitest/coverage-*')` anchored at that
 * bundled dir. So the guard tries the project root FIRST — the supported layout
 * where a directly-declared provider is symlinked at the root, the same copy the
 * bundled vitest walks up to — then falls back to the bundled-vitest anchor,
 * which catches hoisted / pnpm peer-set layouts where the provider lives next to
 * vitest but is not resolvable from the project root (a silent skip otherwise).
 * `@vitest/coverage-*`'s exports map has a `"./*": "./*"` catch-all, so
 * `./package.json` is resolvable. Returns `null` when no anchor can resolve it —
 * Vitest then emits its own (already clear) missing-provider error.
 *
 * The `_createRequire` / `_readFile` parameters let tests inject controlled
 * resolvers without spying on Node's module/fs namespaces.
 */
function readInstalledCoverageProviderVersion(
  providerPackageName: string,
  projectRoot: string,
  _createRequire: (from: string) => { resolve: (id: string) => string } = createRequire,
  _readFile: (path: string) => string = (path) => readFileSync(path, 'utf8'),
): string | null {
  const anchors = [`${projectRoot}/package.json`, bundledVitestAnchor()];
  for (const anchor of anchors) {
    if (!anchor) {
      continue;
    }
    try {
      const req = _createRequire(anchor);
      const pkgJsonPath = req.resolve(`${providerPackageName}/package.json`);
      const parsed = JSON.parse(_readFile(pkgJsonPath)) as { version?: string };
      if (parsed.version) {
        return parsed.version;
      }
    } catch {
      // This anchor can't resolve the provider — try the next one.
    }
  }
  return null;
}

/**
 * Orchestrates the coverage version guard: detect the active provider from
 * Vitest's resolved coverage options, read its installed version from the
 * project root, and throw on a mismatch. A no-op when coverage is off or the
 * provider is not installed.
 *
 * Exported (with injectable `deps`) for unit testing.
 */
export function checkCoverageProviderVersion(
  coverage: { enabled?: boolean; provider?: string } | undefined,
  projectRoot: string,
  deps: {
    createRequire?: (from: string) => { resolve: (id: string) => string };
    readFile?: (path: string) => string;
  } = {},
): void {
  const providerPackageName = resolveCoverageProviderToCheck(coverage);
  if (!providerPackageName) {
    return;
  }
  const installedVersion = readInstalledCoverageProviderVersion(
    providerPackageName,
    projectRoot,
    deps.createRequire,
    deps.readFile,
  );
  assertCoverageProviderVersionMatch(providerPackageName, installedVersion);
}

interface ResolvedCoverageView {
  root: string;
  coverage?: { enabled?: boolean; provider?: string };
}

/**
 * The shared Vitest instance, narrowed to what the late-path guard touches. Its
 * `enableCoverage()` is the programmatic/watch/UI path that turns coverage on
 * AFTER `configureVitest` and creates the provider lazily — wrapping it lets the
 * same skew check guard that path.
 */
interface VitestInstanceView {
  config: ResolvedCoverageView;
  enableCoverage: () => Promise<void>;
}

/**
 * The `configureVitest` plugin hook context, narrowed to the fields the guard
 * reads. Vitest passes more (incl. a per-project `project`), but coverage is a
 * single, GLOBAL runner concern: Vitest creates ONE provider from the shared
 * runner config and imports it from the runner context — never per project. So
 * the guard reads the shared `vitest.config` (its `coverage.{enabled,provider}`
 * already reflects the `--coverage` CLI flags, unlike a `configResolved` Vite
 * hook) and resolves the provider from `vitest.config.root` (the workspace/
 * runner root), which resolves to the exact provider copy Vitest loads. Reading
 * the per-project `project.config.root` would miss a root-level skew or
 * false-positive on a leaf-local provider the runner never loads.
 */
interface VitestConfigureContext {
  vitest: VitestInstanceView;
}

/**
 * Vitest instances the guard has already handled. The `configureVitest` hook
 * fires once PER PROJECT but `vitest` is shared and coverage is global, so this
 * runs the whole guard exactly once per instance (and never leaks, since it is
 * keyed weakly on identity).
 */
const coverageGuardedVitestInstances = new WeakSet<object>();

function vitePlusCoverageVersionGuardPlugin(): PluginOption {
  return {
    name: 'vite-plus:coverage-version-guard',
    // `configureVitest` is a Vitest-specific plugin hook absent from Vite's
    // `Plugin` type (vite-plus-core is a vite fork, so Vitest's `declare module
    // 'vite'` augmentation does not reach it). Cast the object so the extra hook
    // type-checks; Vitest invokes it via `getSortedPluginHooks('configureVitest')`.
    configureVitest(context: VitestConfigureContext) {
      const { vitest } = context;
      // Coverage is global and the provider is imported once from the runner, so
      // run the guard once per shared instance even though this hook fires per
      // project.
      if (coverageGuardedVitestInstances.has(vitest)) {
        return;
      }
      coverageGuardedVitestInstances.add(vitest);

      // STARTUP path: `vitest run --coverage` / config `coverage.enabled` — the
      // only path `vp test`/CI take. `vitest.config.coverage` is the resolved
      // runner config; `vitest.config.root` anchors the provider resolution.
      checkCoverageProviderVersion(vitest.config.coverage, vitest.config.root);

      // LATE path: Vitest's programmatic `enableCoverage()` (node API / watch /
      // UI toggle) flips coverage on after this hook and creates the provider
      // lazily, so the startup check above already returned. Wrap it so the same
      // skew check runs before the provider loads. The wrapper always delegates.
      const enableCoverage = vitest.enableCoverage.bind(vitest);
      vitest.enableCoverage = async () => {
        // `enableCoverage()` sets enabled:true then creates the provider; force
        // enabled so the check does not early-return, and read the provider from
        // the live runner config (the late-path source of truth).
        checkCoverageProviderVersion(
          { enabled: true, provider: vitest.config.coverage?.provider },
          vitest.config.root,
        );
        return enableCoverage();
      };
    },
  } as PluginOption;
}

/**
 * Inject the vitest resolver plugin, the auto-inline matcher plugin, and the
 * coverage version guard into a single inline project config. Used both for
 * root configs and for object-shaped entries inside `test.projects`.
 *
 * The shapes overlap (both have an optional top-level `plugins` array and
 * an optional `test.server.deps.inline`), so a shared helper keeps the
 * wiring consistent.
 */
function injectPluginIntoInlineConfig<
  T extends {
    plugins?: UserConfig['plugins'];
    test?: { server?: { deps?: { inline?: unknown } } };
  },
>(config: T): T {
  return {
    ...config,
    plugins: [
      vitePlusVitestResolverPlugin(),
      vitePlusAutoInlineMatcherPlugin(),
      vitePlusCoverageVersionGuardPlugin(),
      ...(config.plugins ?? []),
    ],
  } as T;
}

/**
 * Walk `config.test?.projects` and inject the vite-plus plugins into each
 * project entry. Vitest spins up an independent Vite pipeline per project, so
 * root-level plugins do NOT propagate — without this, files matched by a
 * project's `include` glob never get the vitest resolver / auto-inline plugins.
 *
 * Entry shapes (from `TestProjectConfiguration`):
 *   - string  (glob path like `'./packages/*'`)  → passed through unchanged.
 *   - object  (inline config with `test: {...}`) → clone and prepend plugin.
 *   - function (sync or async)                   → wrap so its result is injected.
 *   - Promise (resolves to inline config)        → chain `.then(injectPlugin)`.
 *
 * String/glob entries cannot be cloned, so they carry no injected plugin. This
 * only weakens the COVERAGE guard, and only narrowly: coverage is global, and
 * the migration rewrites every nested config file to vite-plus
 * `defineConfig`/`defineProject` (which re-inject the guard), so a migrated
 * workspace still fires it from its resolved projects. The residual gap is a
 * hand-authored workspace whose string globs resolve to raw `vitest/config`
 * sub-configs or bare directory projects — there a provider/runner skew falls
 * back to Vitest's own (softer) warning instead of the guard's hard error.
 */
function injectPluginIntoProject(project: TestProjectConfiguration): TestProjectConfiguration {
  if (typeof project === 'string') {
    return project;
  }
  if (typeof project === 'function') {
    const wrapped: UserProjectConfigFn = (env: ConfigEnv) => {
      const result = project(env);
      if (result instanceof Promise) {
        return result.then(injectPluginIntoInlineConfig);
      }
      return injectPluginIntoInlineConfig(result);
    };
    return wrapped;
  }
  if (project instanceof Promise) {
    return project.then(injectPluginIntoInlineConfig);
  }
  if (typeof project === 'object' && project !== null) {
    return injectPluginIntoInlineConfig(project);
  }
  return project;
}

function injectPlugin(config: UserConfig): UserConfig {
  const injected = injectPluginIntoInlineConfig(config);
  const projects = injected.test?.projects;
  if (!projects || projects.length === 0) {
    return injected;
  }
  return {
    ...injected,
    test: {
      ...injected.test,
      projects: projects.map(injectPluginIntoProject),
    },
  };
}

function injectPluginIntoConfig(config: ViteUserConfigExport): ViteUserConfigExport {
  if (typeof config === 'function') {
    return (env: ConfigEnv) => {
      const result = config(env);
      if (result instanceof Promise) {
        return result.then(injectPlugin);
      }
      return injectPlugin(result);
    };
  }
  if (config instanceof Promise) {
    return config.then(injectPlugin);
  }
  return injectPlugin(config);
}

export function defineConfig(config: UserConfig): UserConfig;
export function defineConfig(config: Promise<UserConfig>): Promise<UserConfig>;
export function defineConfig(config: ViteUserConfigFnObject): ViteUserConfigFnObject;
export function defineConfig(config: ViteUserConfigFnPromise): ViteUserConfigFnPromise;
export function defineConfig(config: ViteUserConfigExport): ViteUserConfigExport;

export function defineConfig(config: ViteUserConfigExport): ViteUserConfigExport {
  return viteDefineConfig(injectPluginIntoConfig(config));
}

/**
 * Inject the vite-plus plugins into a `defineProject` export. A project config
 * (`UserWorkspaceConfig`) cannot itself nest `test.projects`, so this only
 * touches the top-level `plugins` array (no project recursion like
 * [[injectPluginIntoConfig]] does).
 */
function injectPluginIntoProjectExport(config: UserProjectConfigExport): UserProjectConfigExport {
  if (typeof config === 'function') {
    return ((env: ConfigEnv) => {
      const result = config(env);
      return result instanceof Promise
        ? result.then(injectPluginIntoInlineConfig)
        : injectPluginIntoInlineConfig(result);
    }) as UserProjectConfigFn;
  }
  if (config instanceof Promise) {
    return config.then(injectPluginIntoInlineConfig);
  }
  return injectPluginIntoInlineConfig(config);
}

/**
 * `defineProject` counterpart of [[defineConfig]]. A migrated project config
 * that uses `defineProject({ test: { browser: ... } })` — e.g. a file named by
 * a string `test.projects` entry — must still receive the vite-plus resolver /
 * auto-inline plugins, or a browser project can fail to resolve `vitest` /
 * `@vitest/*` from its own root under pnpm strict / Yarn PnP. The raw
 * `vitest/config` helper does not add them.
 */
export function defineProject(config: UserWorkspaceConfig): UserWorkspaceConfig;
export function defineProject(config: Promise<UserWorkspaceConfig>): Promise<UserWorkspaceConfig>;
export function defineProject(config: UserProjectConfigFn): UserProjectConfigFn;
export function defineProject(config: UserProjectConfigExport): UserProjectConfigExport;
export function defineProject(config: UserProjectConfigExport): UserProjectConfigExport {
  return viteDefineProject(injectPluginIntoProjectExport(config) as never);
}

// Number of in-flight `withConfigMetadataResolution` calls, and the marker
// value captured before the first one set it. The marker stays set until the
// LAST overlapping resolution finishes (e.g. several package configs resolved
// concurrently in a monorepo), so an async config that reaches `lazyPlugins`
// while any metadata resolution is still pending is not mistaken for a build.
let configMetadataDepth = 0;
let configMetadataSavedValue: string | undefined;

/**
 * Run a config-metadata resolution (a `resolveConfig` call that loads the
 * user's config purely to read a non-plugin block) with the metadata marker
 * set, so any `lazyPlugins` evaluated during it skips the plugin factory.
 *
 * The marker is scoped in time, not by command name: it is set only while at
 * least one resolution is in flight (ref-counted so overlapping/nested
 * resolutions compose) and the prior value is restored afterwards. By the time
 * the task runner spawns a child (a verbatim `vp run` build, a `vp exec`
 * child), the marker is already gone, so those builds correctly load plugins.
 * Keying on the resolution itself — rather than guessing from the command
 * name — also means command aliases (`vp format`) and not-yet-known commands
 * all load plugins.
 */
export async function withConfigMetadataResolution<T>(fn: () => Promise<T>): Promise<T> {
  if (configMetadataDepth === 0) {
    configMetadataSavedValue = process.env[CONFIG_METADATA_ENV];
    process.env[CONFIG_METADATA_ENV] = '1';
  }
  configMetadataDepth++;
  try {
    return await fn();
  } finally {
    configMetadataDepth--;
    if (configMetadataDepth === 0) {
      if (configMetadataSavedValue === undefined) {
        delete process.env[CONFIG_METADATA_ENV];
      } else {
        process.env[CONFIG_METADATA_ENV] = configMetadataSavedValue;
      }
      configMetadataSavedValue = undefined;
    }
  }
}

export function lazyPlugins(cb: () => PluginOption[]): PluginOption[] | undefined;
export function lazyPlugins(cb: () => Promise<PluginOption[]>): PluginOption[] | undefined;
export function lazyPlugins(
  cb: () => PluginOption[] | Promise<PluginOption[]>,
): PluginOption[] | undefined {
  if (process.env[CONFIG_METADATA_ENV] === '1') {
    return undefined;
  }
  const result = cb();
  if (result instanceof Promise) {
    return [result];
  }
  return result;
}
