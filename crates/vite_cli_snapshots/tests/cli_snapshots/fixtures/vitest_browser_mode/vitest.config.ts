// import { defineProject } from 'vitest/config';
import { playwright } from '@vitest/browser-playwright';

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
