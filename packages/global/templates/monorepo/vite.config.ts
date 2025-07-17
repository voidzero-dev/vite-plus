import { join } from 'node:path';
import { defineConfig } from 'vite-plus';

export default defineConfig({
  resolve: {
    alias: {
      '@scripts': join(import.meta.dirname, 'scripts'),
    },
  },
});
