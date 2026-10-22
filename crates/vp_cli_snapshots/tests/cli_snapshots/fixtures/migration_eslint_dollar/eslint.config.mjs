export default [
  {
    rules: {
      'no-restricted-imports': ['error', { paths: [{ name: 'moment', message: 'moment costs $200 of bundle budget' }] }],
    },
  },
];
