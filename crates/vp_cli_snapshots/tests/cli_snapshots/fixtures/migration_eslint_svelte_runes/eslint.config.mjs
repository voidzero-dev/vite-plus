export default [
  {
    rules: {
      'no-undef': 'error',
    },
  },
  {
    files: ['*.svelte', '**/*.svelte'],
    rules: {
      'no-inner-declarations': 'off',
      'no-self-assign': 'off',
    },
  },
];
