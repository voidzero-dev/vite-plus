import assert from 'node:assert/strict';
import { defineConfig } from 'vite-plus';
import { playwright } from 'vite-plus/test/browser-playwright';

export default defineConfig({
  plugins: [
    {
      name: 'browser-server-port',
      // Vitest replaces a plain api.port: 0 with its default. Override it last.
      enforce: 'post',
      config: {
        order: 'post',
        handler(config) {
          config.test.api.port = 0;
        },
      },
    },
    {
      name: 'vitest-browser-mode-suppress-known-vite-logs',
      configResolved(config) {
        assert.equal(config.test.api.port, 0);
        // "[optimizer]" progress lines fire on a 1s timer, so they only
        // appear on slow cold-cache runs.
        const info = config.logger.info;
        config.logger.info = (message, options) => {
          if (!message.includes('[optimizer]')) {
            info(message, options);
          }
        };
        config.logger.warn = () => {};
      },
    },
  ],
  test: {
    browser: {
      enabled: true,
      provider: playwright(),
      headless: true,
      instances: [
        {
          browser: 'chromium',
        },
      ],
    },
  },
});
