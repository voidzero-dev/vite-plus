import { defineConfig } from '@voidzero-dev/vite-plus-test/config';

import type { OxfmtConfig } from './oxfmt-config';
import type { OxlintConfig } from './oxlint-config';
import type { LibUserConfig } from './lib';

declare module '@voidzero-dev/vite-plus-core' {
  interface UserConfig {
    /**
     * Options for oxlint
     */
    lint?: OxlintConfig;

    fmt?: OxfmtConfig;

    lib?: LibUserConfig | LibUserConfig[];
  }
}

export * from '@voidzero-dev/vite-plus-core';

export * from '@voidzero-dev/vite-plus-test/config';

export { defineConfig };
