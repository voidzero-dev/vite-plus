import { defineConfig } from 'vite-plus';

export default defineConfig({
  fmt: {
    experimentalSortImports: {
      newlinesBetween: false,
    },
    experimentalSortPackageJson: {
      sortScripts: true,
    },
    experimentalTailwindcss: {
      stylesheet: 'client/src/App.css',
    },
    ignorePatterns: [
      'coverage/',
      'dist/',
      '.fate/',
      'client/dist/',
      'client/src/translations/',
      'server/dist/',
      'pnpm-lock.yaml',
    ],
    singleQuote: true,
  },
  lint: {
    extends: ['@nkzw/oxlint-config'],
    ignorePatterns: [
      'coverage',
      'dist',
      '.fate',
      'client/dist',
      'server/dist',
      'server/src/drizzle/migrations/**',
    ],
    options: { typeAware: true, typeCheck: true },
    overrides: [
      {
        files: ['server/src/index.tsx', 'server/src/drizzle/seed.tsx', '**/__tests__/**'],
        rules: {
          'no-console': 'off',
        },
      },
    ],
    rules: {
      '@typescript-eslint/no-explicit-any': 'off',
    },
  },
});
