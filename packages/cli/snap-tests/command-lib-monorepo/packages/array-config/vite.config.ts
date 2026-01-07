export default {
  lib: [{
    entry: ['./src/sub/index.ts'],
    clean: true,
    format: ['esm'],
    minify: false,
    dts: true,
    outDir: './dist',
  }],
};
