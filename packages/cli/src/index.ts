import {
  defineConfig as viteDefineConfig,
  type Plugin as VitestPlugin,
} from '@voidzero-dev/vite-plus-test/config';

import type { LibUserConfig } from './lib';
import type { FormatOptions } from './oxfmt-config';
import type { OxlintConfig } from './oxlint-config';
import type { Tasks } from './task-config';

declare module '@voidzero-dev/vite-plus-core' {
  interface UserConfig {
    /**
     * Options for oxlint
     */
    lint?: OxlintConfig;

    fmt?: FormatOptions;

    lib?: LibUserConfig | LibUserConfig[];

    tasks?: Tasks;

    // temporary solution to load plugins lazily
    // We need to support this in the upstream vite
    lazy?: () => Promise<{
      plugins?: VitestPlugin[];
    }>;
  }
}

export * from '@voidzero-dev/vite-plus-core';

export * from '@voidzero-dev/vite-plus-test/config';

// @ts-expect-error - skip overriding the types in vite-plus
export const defineConfig: typeof viteDefineConfig = (config) => {
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
  }
  return viteDefineConfig(config);
};
