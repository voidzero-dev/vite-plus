import { readFileSync } from 'node:fs';
import { defineConfig } from 'vite-plus';

const oxlintConfig = JSON.parse(readFileSync(new URL('./.oxlintrc.json', import.meta.url), 'utf8'));
const oxfmtConfig = JSON.parse(readFileSync(new URL('./.oxfmtrc.json', import.meta.url), 'utf8'));

export default defineConfig({
  lint: {
    ...oxlintConfig,
    ignorePatterns: ['dist/**'],
  },
  fmt: {
    ...oxfmtConfig,
    ignorePatterns: ['dist/**'],
  },
});
