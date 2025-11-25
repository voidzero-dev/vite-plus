import { defineConfig } from '@voidzero-dev/vite-plus';

export default defineConfig({
  lint: {
    rules: {
      'no-console': 'warn',
    },
  },
  test: {
    exclude: [
      '**/node_modules/**',
      '**/snap-tests/**',
      './rolldown/**',
      './rolldown-vite/**',
      // FIXME: Error: failed to prepare the command for injection: Invalid argument (os error 22)
      'packages/cli/binding/__tests__/',
    ],
  },
});
