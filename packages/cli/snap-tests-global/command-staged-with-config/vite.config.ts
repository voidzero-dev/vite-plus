export default {
  lint: {
    rules: {
      'no-eval': 'error',
    },
  },
  fmt: {},
  staged: {
    '*.ts': 'vp check --fix',
    '*.js': 'vp lint',
  },
};
