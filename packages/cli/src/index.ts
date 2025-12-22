import type { defineConfig as defineLibConfig } from '@voidzero-dev/vite-plus-core/lib';
import { defineConfig } from '@voidzero-dev/vite-plus-test/config';

import type { OxfmtConfig } from './oxfmt-config';
import type { OxlintConfig } from './oxlint-config';

declare module '@voidzero-dev/vite-plus-core' {
  interface UserConfig {
    /**
     * Options for oxlint
     */
    lint?: OxlintConfig;

    fmt?: OxfmtConfig;

    lib?: Parameters<typeof defineLibConfig>[0];
  }
}

export * from '@voidzero-dev/vite-plus-core';

export * from '@voidzero-dev/vite-plus-test/config';

export { defineConfig };
