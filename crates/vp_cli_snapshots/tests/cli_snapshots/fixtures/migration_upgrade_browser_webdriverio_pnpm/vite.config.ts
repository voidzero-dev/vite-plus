import { defineConfig } from 'vite-plus';
import { webdriverio } from 'vite-plus/test/browser-webdriverio';

export default defineConfig({
  test: {
    browser: {
      enabled: true,
      provider: webdriverio(),
    },
  },
});
