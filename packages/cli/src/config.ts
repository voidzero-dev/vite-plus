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

declare module '@voidzero-dev/vite-plus/vite' {
  interface UserConfig {
    /**
     * Options for oxlint
     */
    lint?: LintConfig;

    fmt?: FmtConfig;
  }
}

// @ts-expect-error
export * from './vitest/config';
