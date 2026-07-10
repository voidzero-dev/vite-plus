// Flat config exercising the type-aware sniffer without importing
// typescript-eslint at runtime, so `@oxlint/migrate` can load the file
// in the PTY snapshot sandbox where no node_modules are installed.
export default [
  {
    languageOptions: {
      parserOptions: {
        projectService: true,
      },
    },
    rules: {
      'no-unused-vars': 'error',
    },
  },
];
