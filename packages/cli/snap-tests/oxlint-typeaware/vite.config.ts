export default {
  lint: {},
  run: {
    tasks: {
      lint: {
        command: 'vp lint ./src',
      },
      'lint-typeaware': {
        command: 'vp lint --type-aware ./src',
      },
    },
  },
};
