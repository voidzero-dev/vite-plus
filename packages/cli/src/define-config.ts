import {
  defineConfig as viteDefineConfig,
  type ConfigEnv,
} from '@voidzero-dev/vite-plus-test/config';

import type { UserConfig } from './index.js';

type ViteUserConfigFnObject = (env: ConfigEnv) => UserConfig;
type ViteUserConfigFnPromise = (env: ConfigEnv) => Promise<UserConfig>;
type ViteUserConfigFn = (env: ConfigEnv) => UserConfig | Promise<UserConfig>;
type ViteUserConfigExport =
  | UserConfig
  | Promise<UserConfig>
  | ViteUserConfigFnObject
  | ViteUserConfigFnPromise
  | ViteUserConfigFn;

export function defineConfig(config: UserConfig): UserConfig;
export function defineConfig(config: Promise<UserConfig>): Promise<UserConfig>;
export function defineConfig(config: ViteUserConfigFnObject): ViteUserConfigFnObject;
export function defineConfig(config: ViteUserConfigFnPromise): ViteUserConfigFnPromise;
export function defineConfig(config: ViteUserConfigExport): ViteUserConfigExport;

export function defineConfig(config: ViteUserConfigExport): ViteUserConfigExport {
  if (typeof config === 'object') {
    if (config instanceof Promise) {
      return config.then((config) => {
        if (config.lazy) {
          return config.lazy().then(({ plugins }) =>
            viteDefineConfig({
              ...config,
              plugins: [...(config.plugins || []), ...(plugins || [])],
            }),
          );
        }
        return viteDefineConfig(config);
      });
    } else if (config.lazy) {
      return config.lazy().then(({ plugins }) =>
        viteDefineConfig({
          ...config,
          plugins: [...(config.plugins || []), ...(plugins || [])],
        }),
      );
    }
  } else if (typeof config === 'function') {
    return viteDefineConfig((env) => {
      const c = config(env);
      if (c instanceof Promise) {
        return c.then((v) => {
          if (v.lazy) {
            return v
              .lazy()
              .then(({ plugins }) =>
                viteDefineConfig({ ...v, plugins: [...(v.plugins || []), ...(plugins || [])] }),
              );
          }
          return v;
        });
      }
      if (c.lazy) {
        return c
          .lazy()
          .then(({ plugins }) => ({ ...c, plugins: [...(c.plugins || []), ...(plugins || [])] }));
      }
      return c;
    });
  }
  return viteDefineConfig(config);
}
