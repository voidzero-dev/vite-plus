import { defineConfig } from '@voidzero-dev/vite-plus';

export default defineConfig({
  test: {
    include: ['**/__tests__/**/*.spec.ts'],
    exclude: [
      'packages/global/templates',
      // ignore __tests__ at node_modules, e.g.: packages/cli/node_modules/@napi-rs/cli/src/utils/__tests__/typegen.spec.ts
      '**/node_modules',
    ],
  },
});
