import { defineConfig } from 'vite-plus';

export default defineConfig({
  lint: {
    rules: {
      'no-console': 'off',
    },
  },
  fmt: {
    singleQuote: true,
  },
});
