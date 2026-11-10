import { defineConfig } from 'vite-plus';
export default defineConfig({
  migrate: { ignorePatterns: ['/templates/*', '!/templates/managed/', '/examples/'] },
  lint: {
    jsPlugins: ['vite-plus/oxlint-plugin'],
    rules: { 'vite-plus/prefer-vite-plus-imports': 'error', 'no-debugger': 'error' },
  },
});
