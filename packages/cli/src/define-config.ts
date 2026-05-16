import type { PluginOption, UserConfig } from '@voidzero-dev/vite-plus-core';
import type { OxfmtConfig } from 'oxfmt';
import type { OxlintConfig } from 'oxlint';
import {
  defineConfig as viteDefineConfig,
  type ConfigEnv,
  type TestProjectConfiguration,
  type TestProjectInlineConfiguration,
  type UserProjectConfigFn,
} from 'vitest/config';
import type { InlineConfig as VitestInlineConfig } from 'vitest/node';

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
       * `createConfig.templates` manifest.
       */
      defaultTemplate?: string;
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
 * Rewrite bare-root `vite-plus/test` import specifiers to `vitest` so that
 * `@vitest/mocker`'s static hoister (which hardcodes `hoistedModule = "vitest"`)
 * recognizes calls like `vi.mock(...)`. Subpaths such as
 * `vite-plus/test/browser` are intentionally left unchanged.
 *
 * Task #50 pins `vitest` and the `@vitest/*` family so both specifiers resolve
 * to the same physical module, making this rewrite runtime-safe.
 *
 * Exported for unit testing.
 */
export function rewriteVitePlusTestSpecifier(code: string): string {
  if (!code.includes('vite-plus/test')) {
    return code;
  }
  return code
    .replace(/(from\s+['"])vite-plus\/test(?=['"])/g, '$1vitest')
    .replace(/(import\s*\(\s*['"])vite-plus\/test(?=['"])/g, '$1vitest')
    .replace(/(require\s*\(\s*['"])vite-plus\/test(?=['"])/g, '$1vitest');
}

function vitePlusTestSpecifierRewritePlugin(): PluginOption {
  return {
    name: 'vite-plus:vitest-specifier-rewrite',
    enforce: 'pre',
    transform(code, id) {
      if (id.includes('/node_modules/')) {
        return null;
      }
      const newCode = rewriteVitePlusTestSpecifier(code);
      if (newCode === code) {
        return null;
      }
      return { code: newCode, map: null };
    },
  };
}

/**
 * Inject the rewrite plugin into a single inline project config. Used both
 * for root configs and for object-shaped entries inside `test.projects`.
 *
 * The shapes overlap (both have an optional top-level `plugins` array), so a
 * shared helper keeps the wiring consistent.
 */
function injectPluginIntoInlineConfig<T extends { plugins?: UserConfig['plugins'] }>(config: T): T {
  return {
    ...config,
    plugins: [vitePlusTestSpecifierRewritePlugin(), ...(config.plugins ?? [])],
  };
}

/**
 * Walk `config.test?.projects` and inject the rewrite plugin into each
 * project entry. Vitest spins up an independent Vite pipeline per project, so
 * root-level plugins do NOT propagate — without this, files matched by a
 * project's `include` glob never get the `vite-plus/test` → `vitest` rewrite.
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
    const fn = project as UserProjectConfigFn;
    const wrapped: UserProjectConfigFn = (env: ConfigEnv) => {
      const result = fn(env);
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
    return injectPluginIntoInlineConfig(project as TestProjectInlineConfiguration);
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
