// import { defineProject } from 'vitest/config';
import { playwright } from 'vite-plus/test/browser-playwright';

export default {
  plugins: [
    {
      name: 'vitest-browser-mode-suppress-warnings',
      configResolved(config) {
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
