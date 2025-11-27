import { join } from 'node:path';

import { foo } from '@foo/vite-plugin-foo';
import { playwright } from '@vitest/browser-playwright';
import { defineConfig } from 'vitest/config';

export default defineConfig({
  plugins: [foo()],
  test: {
    dir: join(import.meta.dirname, 'test'),
    browser: {
      enabled: true,
      provider: playwright(),
      headless: true,
      screenshotFailures: false,
      instances: [{ browser: 'chromium' }],
    },
  },
});
