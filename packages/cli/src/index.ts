import {
  defineConfig as defineViteConfig,
  type UserConfig,
  type UserConfigExport,
  type UserConfigFn,
  type UserConfigFnObject,
  type UserConfigFnPromise,
} from 'rolldown-vite';
import { type ViteUserConfig } from 'vitest/config';

type ExtendsConfig<T> = T extends UserConfig ? T & { extends?: string; test?: ViteUserConfig['test'] }
  : T extends Promise<UserConfig> ? Promise<UserConfig & { extends?: string; test?: ViteUserConfig['test'] }>
  : T extends UserConfigFnObject ? T & { extends?: string; test?: ViteUserConfig['test'] }
  : T extends UserConfigFnPromise ? Promise<UserConfig & { extends?: string; test?: ViteUserConfig['test'] }>
  : T extends UserConfigFn ? T & { extends?: string; test?: ViteUserConfig['test'] }
  : T extends UserConfigExport ? UserConfigExport
  : T;

function defineConfig(
  config: UserConfig & { extends?: string; test?: ViteUserConfig['test'] },
): UserConfig;
function defineConfig(
  config: Promise<
    UserConfig & { extends?: string; test?: ViteUserConfig['test'] }
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

export { defineConfig };
