export default {
  staged: {
    '*.ts': 'vp check --fix',
    '*.js': 'node fail.mjs',
    '*.out': 'node output.mjs',
  },
};
