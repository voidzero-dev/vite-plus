import { defineConfig } from 'vite-plus';

export default defineConfig({
  cacheDir: '.test-cache/results',
  test: {
    fsModuleCachePath: '.test-cache/root',
    maxWorkers: 1,
    projects: [
      {
        test: {
          name: 'root-cache',
          include: ['src/**/*.test.ts'],
          slowTestThreshold: 60_000,
          sequence: { groupOrder: 0 },
        },
      },
      {
        test: {
          name: 'project-cache',
          include: ['src/**/*.test.ts'],
          slowTestThreshold: 60_000,
          fsModuleCachePath: '.test-cache/project',
          sequence: { groupOrder: 1 },
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
