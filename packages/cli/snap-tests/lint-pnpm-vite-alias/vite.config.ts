import { defineConfig } from 'vite-plus';

export default defineConfig({
  lint: {
    jsPlugins: [{ name: 'vite-plus', specifier: 'vite-plus/oxlint-plugin' }],
    rules: {
      'vite-plus/require-pnpm-vite-alias': 'error',
    },
  },
});
