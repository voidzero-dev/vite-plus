export default {
  lint: {
    jsPlugins: ['vite-plus/oxlint-plugin'],
    rules: { 'vite-plus/prefer-vite-plus-imports': 'error' },
    overrides: [
      { files: ['templates/**'], rules: { 'vite-plus/prefer-vite-plus-imports': 'off' } },
    ],
  },
};
