import { defineConfig } from 'vite-plus';

export default defineConfig({
  test: {
    include: ['src/**/*.test.ts'],
    slowTestThreshold: 60_000,
  },
  run: {
    tasks: {
      test: { command: 'vp test run' },
    },
  },
});
