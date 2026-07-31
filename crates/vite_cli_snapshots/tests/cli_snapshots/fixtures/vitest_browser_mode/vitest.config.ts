// import { defineProject } from 'vitest/config';
import { playwright } from 'vite-plus/test/browser-playwright';

export default {
  plugins: [
    {
      name: 'vitest-browser-mode-suppress-vite-logs',
      configResolved(config) {
        config.logger.info = () => {};
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
};
