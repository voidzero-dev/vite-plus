import { defineConfig } from 'cf/config';

export default defineConfig({
  worker: {
    name: 'viteplus-dev',
    compatibilityDate: '2026-09-02',
    assets: {
      htmlHandling: 'auto-trailing-slash',
      notFoundHandling: '404-page',
    },
  },
});
