import {
  type ConfigEnv,
  defineConfig as defineViteConfig,
  resolveConfig as resolveViteConfig,
  type ResolvedConfig as ViteResolvedConfig,
  type UserConfig,
  type UserConfigExport,
  type UserConfigFn,
  type UserConfigFnObject,
  type UserConfigFnPromise,
} from 'vite';
import { type ViteUserConfig } from 'vitest/config';

// replace it with the real oxlint config in the future
export type LintConfig = {
  rules: {
    [key: string]: string;
  };
};

export type FmtConfig = {
  rules: {
    [key: string]: string;
  };
};

type VitePlusConfig = {
  extends?: string;
  test?: ViteUserConfig['test'];
  lint?: LintConfig;
  fmt?: FmtConfig;
};

type ExtendsConfig<T> = T extends UserConfig ? T & VitePlusConfig
  : T extends Promise<UserConfig> ? Promise<UserConfig & VitePlusConfig>
  : T extends UserConfigFnObject ? (env: ConfigEnv) => UserConfig & VitePlusConfig
  : T extends UserConfigFnPromise ? Promise<
      UserConfig & {
        extends?: string;
        test?: ViteUserConfig['test'];
        lint?: LintConfig;
        fmt?: FmtConfig;
      }
    >
  : T extends UserConfigFn ? T & VitePlusConfig
  : T extends UserConfigExport ? UserConfigExport
  : T;

type ResolvedConfig = ViteResolvedConfig & {
  lint?: LintConfig;
  fmt?: FmtConfig;
};

function defineConfig(
  config: UserConfig & VitePlusConfig,
): UserConfig & VitePlusConfig;
function defineConfig(
  config: Promise<UserConfig & VitePlusConfig>,
): Promise<UserConfig & VitePlusConfig>;
function defineConfig(
  config: ExtendsConfig<UserConfigFnObject>,
): UserConfigFnObject;
function defineConfig(
  config: ExtendsConfig<UserConfigFnPromise>,
): UserConfigFnPromise;
function defineConfig(config: ExtendsConfig<UserConfigFn>): UserConfigFn;
function defineConfig(config: ExtendsConfig<UserConfigExport>): UserConfigExport;
function defineConfig(
  config: ExtendsConfig<UserConfigExport>,
): UserConfigExport {
  return defineViteConfig(config);
}

function resolveConfig(
  config: UserConfig & VitePlusConfig,
  command: 'build' | 'serve',
  defaultMode?: string,
  defaultNodeEnv?: string,
  isPreview?: boolean,
): Promise<ResolvedConfig> {
  return resolveViteConfig(
    config,
    command,
    defaultMode,
    defaultNodeEnv,
    isPreview,
  );
}

export * from 'vite';
export { defineConfig, resolveConfig, type ResolvedConfig };
