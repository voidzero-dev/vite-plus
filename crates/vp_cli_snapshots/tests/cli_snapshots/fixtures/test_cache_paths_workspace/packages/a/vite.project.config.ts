import { defineConfig } from 'vite-plus';

export default defineConfig({
  test: {
    // Keep failure details and results on stdout for stable snapshots on Windows.
    reporters: ['tap'],
    fsModuleCachePath: '../../.test-cache/root',
    projects: [
      {
        test: {
          name: 'empty-project-cache',
          fsModuleCachePath: '',
          include: ['src/**/*.test.ts'],
          slowTestThreshold: 60_000,
        },
      },
    ],
  },
  run: {
    tasks: {
      test: { command: 'vp test run' },
    },
  },
});
