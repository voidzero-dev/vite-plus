// Mock vite config for testing workspace root config resolution
// When this config is read, fmt will use singleQuote: true (file uses single quotes → passes)
// Without this config, fmt uses default singleQuote: false (double quotes → fails)
export default {
  lint: {
    rules: {
      'no-console': 'error',
      'no-debugger': 'warn',
    },
  },
  fmt: {
    singleQuote: true,
  },
};
