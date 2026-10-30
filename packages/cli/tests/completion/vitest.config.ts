import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    include: ['packages/cli/tests/completion/**/*.integration.ts'],
    // A receipt case starts setup and completion in separate bounded native processes.
    testTimeout: 45_000,
  },
});
