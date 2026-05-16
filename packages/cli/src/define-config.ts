import type { PluginOption, UserConfig } from '@voidzero-dev/vite-plus-core';
import type { OxfmtConfig } from 'oxfmt';
import type { OxlintConfig } from 'oxlint';
import { defineConfig as viteDefineConfig, type ConfigEnv } from 'vitest/config';
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

function injectPlugin(config: UserConfig): UserConfig {
  return {
    ...config,
    plugins: [vitePlusTestSpecifierRewritePlugin(), ...(config.plugins ?? [])],
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
