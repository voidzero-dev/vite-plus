import { join } from 'node:path';

import { defineConfig } from '@voidzero-dev/vite-plus';

export default defineConfig({
  extends: '../../vite.config.ts',
  resolve: {
    alias: {
      '@log': `${join(import.meta.dirname, 'src')}`,
    },
  },
  test: {
    reporters: ['default'],
  },
});
