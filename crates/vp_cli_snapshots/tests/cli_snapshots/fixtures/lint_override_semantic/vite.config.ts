import { defineConfig } from 'vite-plus';

export default defineConfig({
  lint: {
    rules: { 'no-console': 'error' },
    plugins: ['unicorn', 'eslint'],
    overrides: [
      {
        files: ['**/*.vue'],
        plugins: ['vue'],
        rules: {
          'vue/no-export-in-script-setup': 'error',
        },
      },
    ],
  },
});
