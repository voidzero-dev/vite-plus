import { defineConfig } from 'vite-plus';

export default defineConfig({
  staged: {
    '*.js': 'vp check --fix',
  },
});
