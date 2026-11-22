import { defineConfig } from 'vite';

export default defineConfig({
  lint: {
    options: {
      typeAware: false,
      typeCheck: false,
      reportUnusedDisableDirectives: 'warn',
    },
  },
});
