import cloudflare from '@cloudflare/oxlint-config/vite-plus';
import { defineConfig } from 'vite-plus';

export default defineConfig({
  lint: {
    extends: [cloudflare],
    env: cloudflare.env,
    ignorePatterns: cloudflare.ignorePatterns,
    options: { typeCheck: true },
    jsPlugins: [{ name: 'vite-plus', specifier: 'vite-plus/oxlint-plugin' }],
    rules: {
      'typescript/no-non-null-assertion': 'off',
      'vite-plus/prefer-vite-plus-imports': 'error',
    },
  },
  run: {
    cache: true,
  },
});
