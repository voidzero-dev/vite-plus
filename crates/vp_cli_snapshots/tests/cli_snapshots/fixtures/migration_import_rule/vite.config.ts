import { defineConfig } from 'vite-plus';
export default defineConfig({
  lint: {
    jsPlugins: ['vite-plus/oxlint-plugin'],
    rules: { 'vite-plus/prefer-vite-plus-imports': 'error', 'no-debugger': 'error' },
    overrides: [
      { files: ['templates/**', 'examples/**'], rules: { 'vite-plus/prefer-vite-plus-imports': 'off' } },
      { files: ['templates/managed/**'], rules: { 'vite-plus/prefer-vite-plus-imports': 'warn' } },
    ],
  },
});
