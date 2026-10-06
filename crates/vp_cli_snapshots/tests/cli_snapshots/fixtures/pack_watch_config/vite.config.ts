export default {
  pack: {
    entry: ['index.ts'],
    dts: false,
    minify: false,
    onSuccess() {
      process.send?.('built');
    },
  },
};
