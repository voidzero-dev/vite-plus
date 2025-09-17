import {
  defineConfig as defineViteConfig,
  resolveConfig as resolveViteConfig,
  type ResolvedConfig,
  type UserConfig,
  type UserConfigExport,
  type UserConfigFn,
  type UserConfigFnObject,
  type UserConfigFnPromise,
} from 'rolldown-vite';
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

type ExtendsConfig<T> = T extends UserConfig ? T & {
    extends?: string;
    test?: ViteUserConfig['test'];
    lint?: LintConfig;
    fmt?: FmtConfig;
  }
  : T extends Promise<UserConfig> ? Promise<
      UserConfig & {
        extends?: string;
        test?: ViteUserConfig['test'];
        lint?: LintConfig;
        fmt?: FmtConfig;
      }
    >
  : T extends UserConfigFnObject ? T & {
      extends?: string;
      test?: ViteUserConfig['test'];
      lint?: LintConfig;
      fmt?: FmtConfig;
    }
  : T extends UserConfigFnPromise ? Promise<
      UserConfig & {
        extends?: string;
        test?: ViteUserConfig['test'];
        lint?: LintConfig;
        fmt?: FmtConfig;
      }
    >
  : T extends UserConfigFn ? T & {
      extends?: string;
      test?: ViteUserConfig['test'];
      lint?: LintConfig;
      fmt?: FmtConfig;
    }
  : T extends UserConfigExport ? UserConfigExport
  : T;

type UniversalResolvedConfig = ResolvedConfig & {
  lint?: LintConfig;
  fmt?: FmtConfig;
};

function defineConfig(
  config: UserConfig & {
    extends?: string;
    test?: ViteUserConfig['test'];
    lint?: LintConfig;
    fmt?: FmtConfig;
  },
): UserConfig;
function defineConfig(
  config: Promise<
    UserConfig & {
      extends?: string;
      test?: ViteUserConfig['test'];
      lint?: LintConfig;
      fmt?: FmtConfig;
    }
  >,
): Promise<UserConfig>;
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
  config: UserConfig & {
    extends?: string;
    test?: ViteUserConfig['test'];
    lint?: LintConfig;
    fmt?: FmtConfig;
  },
  command: 'build' | 'serve',
  defaultMode?: string,
  defaultNodeEnv?: string,
  isPreview?: boolean,
): Promise<UniversalResolvedConfig> {
  return resolveViteConfig(config, command, defaultMode, defaultNodeEnv, isPreview);
}

export { defineConfig, resolveConfig };
