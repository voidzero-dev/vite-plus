import { defineConfig } from 'vite-plus';

export default defineConfig({
  test: {
    experimental: { diagnostics: false },
    name: 'root-cache',
    include: ['src/**/*.test.ts'],
    slowTestThreshold: 60_000,
    maxWorkers: 1,
    sequence: { groupOrder: 0 },
  },
  plugins: [
    {
      name: 'inject-cache-project',
      configureVitest: {
        order: 'pre',
        async handler({ injectTestProjects }) {
          await injectTestProjects({
            extends: false,
            test: {
              name: 'injected-cache',
              include: ['src/**/*.test.ts'],
              slowTestThreshold: 60_000,
              fsModuleCachePath: '.test-cache/injected',
              sequence: { groupOrder: 1 },
            },
          });
        },
      },
    },
  ],
  run: {
    tasks: {
      test: { command: 'vp test run' },
    },
  },
});
