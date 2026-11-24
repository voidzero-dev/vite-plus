import cloudflare from '@cloudflare/oxlint-config/vite-plus';
import { defineConfig } from 'vite-plus';

export default defineConfig({
  lint: {
    extends: [cloudflare],
    env: cloudflare.env,
    ignorePatterns: cloudflare.ignorePatterns,
    options: { typeCheck: true },
    jsPlugins: [{ name: 'vite-plus', specifier: 'vite-plus/oxlint-plugin' }],
    rules: { 'vite-plus/prefer-vite-plus-imports': 'error' },
    overrides: [
      {
        // create-vite's vanilla-ts entry point asserts that its two DOM elements exist.
        files: ['**/*/src/main.ts'],
        rules: { 'typescript/no-non-null-assertion': 'off' },
      },
    ],
  },
  run: {
    cache: true,
  },
});
