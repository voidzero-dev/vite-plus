export const staged = {
  '*.txt': 'node verify-staged.mjs',
};

export const pack = {
  entry: ['app/index.js'],
  dts: false,
  format: 'esm',
};
