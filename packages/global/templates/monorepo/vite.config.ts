import { defineConfig } from '@voidzero-dev/vite-plus';
import { join } from 'node:path';

export default defineConfig({
  resolve: {
    alias: {
      '@scripts': join(import.meta.dirname, 'scripts'),
    },
  },
});
