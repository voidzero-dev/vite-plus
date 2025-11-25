import { defineConfig } from '@voidzero-dev/vite-plus-test/config';

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

declare module '@voidzero-dev/vite-plus-core' {
  interface UserConfig {
    /**
     * Options for oxlint
     */
    lint?: LintConfig;

    fmt?: FmtConfig;
  }
}

export * from '@voidzero-dev/vite-plus-core';

export { defineConfig };
