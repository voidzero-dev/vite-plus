import { type Plugin as VitestPlugin } from '@voidzero-dev/vite-plus-test/config';

import { defineConfig } from './define-config.js';
import type { LibUserConfig } from './lib';
import type { FormatOptions } from './oxfmt-config';
import type { OxlintConfig } from './oxlint-config';
import type { RunConfig } from './run-config';

declare module '@voidzero-dev/vite-plus-core' {
  interface UserConfig {
    /**
     * Options for oxlint
     */
    lint?: OxlintConfig;

    fmt?: FormatOptions;

    lib?: LibUserConfig | LibUserConfig[];

    run?: RunConfig;

    // temporary solution to load plugins lazily
    // We need to support this in the upstream vite
    lazy?: () => Promise<{
      plugins?: VitestPlugin[];
    }>;
  }
}

export * from '@voidzero-dev/vite-plus-core';

export * from '@voidzero-dev/vite-plus-test/config';

export { defineConfig };
