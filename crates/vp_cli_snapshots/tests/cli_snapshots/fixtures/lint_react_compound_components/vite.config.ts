export default {
  lint: {
    plugins: ['react'],
    categories: { correctness: 'off' },
    rules: {
      'react/only-export-components': ['error', { allowCompoundComponents: true }],
    },
  },
};
