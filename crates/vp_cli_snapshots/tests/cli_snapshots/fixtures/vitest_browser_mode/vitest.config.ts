// import { defineProject } from 'vitest/config';
import { defineConfig } from 'vite-plus';
import { playwright } from 'vite-plus/test/browser-playwright';

// `defineConfig` (not a bare object literal) is what injects the Vite+ plugins,
// including the one that keeps `vite-plus/test` out of dependency pre-bundling.
// Vitest 5 pre-bundles it otherwise, which hands the test file a second copy of
// the runtime whose runner state is unset — the first `describe()` then fails
// with "Cannot read properties of undefined (reading 'config')".
export default defineConfig({
  plugins: [
    {
      name: 'vitest-browser-mode-suppress-known-vite-logs',
      configResolved(config) {
        // "is in use, trying another one": port fallback depends on other
        // concurrently running servers. "[optimizer]" progress lines fire on
        // a 1s timer, so they only appear on slow cold-cache runs.
        const nonDeterministicLogs = [
          'is in use, trying another one',
          '[optimizer]',
        ];
        const info = config.logger.info;
        config.logger.info = (message, options) => {
          if (!nonDeterministicLogs.some((log) => message.includes(log))) {
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
