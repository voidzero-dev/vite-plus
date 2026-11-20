import { defineConfig } from 'vite-plus';
export default defineConfig({
  lint: {
    jsPlugins: [{ name: 'vite-plus', specifier: 'vite-plus/oxlint-plugin' }],
    rules: { 'vite-plus/prefer-vite-plus-imports': 'error', 'no-debugger': 'error' },
    overrides: [
      {
        files: ['templates/**', 'examples/**'],
        rules: { 'vite-plus/prefer-vite-plus-imports': 'off' },
      },
    ],
  },
});
