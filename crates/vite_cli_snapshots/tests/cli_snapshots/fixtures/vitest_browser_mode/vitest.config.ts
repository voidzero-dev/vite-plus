// import { defineProject } from 'vitest/config';
import { playwright } from 'vite-plus/test/browser-playwright';

export default {
  plugins: [
    {
      name: 'vitest-browser-mode-suppress-known-vite-logs',
      configResolved(config) {
        const info = config.logger.info;
        config.logger.info = (message, options) => {
          if (!message.includes('is in use, trying another one')) {
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
};
