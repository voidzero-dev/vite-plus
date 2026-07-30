// import { defineProject } from 'vitest/config';
import { playwright } from 'vite-plus/test/browser-playwright';

export default {
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
