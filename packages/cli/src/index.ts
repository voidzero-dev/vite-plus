import { defineConfig } from '@voidzero-dev/vite-plus-test/config';

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
  }
}

export * from '@voidzero-dev/vite-plus-core';

export * from '@voidzero-dev/vite-plus-test/config';

export { defineConfig };
