import { type Plugin as VitestPlugin } from '@voidzero-dev/vite-plus-test/config';
import type { OxfmtConfig } from 'oxfmt';
import type { OxlintConfig } from 'oxlint';

import { defineConfig } from './define-config.js';
import type { PackUserConfig } from './pack.js';
import type { RunConfig } from './run-config.js';
import type { StagedConfig } from './staged-config.js';

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
