import { defineConfig } from '@voidzero-dev/vite-plus/config';

export default defineConfig({
  lint: {
    rules: {
      'no-console': 'warn',
    },
  },
  test: {
    exclude: [
      '**/node_modules/**',
      './rolldown/**',
      './rolldown-vite/**',
    ],
  },
});
