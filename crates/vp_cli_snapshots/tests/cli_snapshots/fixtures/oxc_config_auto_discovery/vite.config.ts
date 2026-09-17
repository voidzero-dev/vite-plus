import { defineConfig } from 'vite-plus';

export default defineConfig({
  lint: {
    rules: {
      'no-console': 'error',
    },
  },
  fmt: {
    singleQuote: true,
    semi: false,
  },
});
