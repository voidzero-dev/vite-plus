import { defineConfig } from 'rolldown';

export default defineConfig({
  input: './src/index.ts',
  external: [/^node:/, 'oxfmt', 'oxlint', /rolldown-vite/],
  output: {
    format: 'esm',
    dir: './dist',
  },
});
