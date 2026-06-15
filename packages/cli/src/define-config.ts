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

declare module '@voidzero-dev/vite-plus-core' {
  interface UserConfig {
    /**
     * Options for oxlint
     */
    lint?: OxlintConfig;

    fmt?: OxfmtConfig;

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
 *     with the bundled runner; Vitest validates provider/runner versions and
 *     errors on a mismatch.
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

/**
 * Inject the vitest resolver plugin and the auto-inline matcher plugin into a
 * single inline project config. Used both for root configs and for
 * object-shaped entries inside `test.projects`.
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

const VITE_COMMANDS = new Set(['dev', 'build', 'test', 'preview']);

export function lazyPlugins(cb: () => PluginOption[]): PluginOption[] | undefined;
export function lazyPlugins(cb: () => Promise<PluginOption[]>): PluginOption[] | undefined;
export function lazyPlugins(
  cb: () => PluginOption[] | Promise<PluginOption[]>,
): PluginOption[] | undefined {
  const cmd = process.env.VP_COMMAND;
  if (!cmd || VITE_COMMANDS.has(cmd)) {
    const result = cb();
    if (result instanceof Promise) {
      return [result];
    }
    return result;
  }
  return undefined;
}
