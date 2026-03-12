export default {
  lint: {
    rules: {
      'no-eval': 'error',
    },
  },
  staged: {
    '*.ts': 'vp check --fix',
    '*.js': 'vp lint',
  },
};
