import { defineConfig } from 'vite-plus';

export default defineConfig({
  test: {
    // Keep passing test details independent of machine load in snapshots.
    slowTestThreshold: 60_000,
  },
  run: {
    tasks: {
      test: { command: 'vp test run' },
    },
  },
});
