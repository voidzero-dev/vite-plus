// Mock vite config for testing workspace root config resolution
export default {
  lint: {
    rules: {
      'no-console': 'error',
      'no-debugger': 'warn',
    },
  },
  fmt: {
    rules: {
      indentWidth: 4,
      lineWidth: 100,
    },
  },
};
