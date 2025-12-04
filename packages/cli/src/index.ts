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
  }
}

export * from '@voidzero-dev/vite-plus-core';

export { defineConfig };
// TODO: how to keep sync with vitest exports?
export {
  configDefaults,
  coverageConfigDefaults,
  defaultExclude,
  defaultInclude,
  defaultBrowserPort,
  defineProject,
} from '@voidzero-dev/vite-plus-test/config';
