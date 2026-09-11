import { defineConfig } from 'vite-plus';

export default defineConfig({
  lint: {
    jsPlugins: [
      './lint/plugin.js',
      { name: 'vite-plus', specifier: 'vite-plus/oxlint-plugin' },
    ],
    rules: {
      'local/no-foo': 'error',
      'vite-plus/prefer-vite-plus-imports': 'error',
    },
  },
});
