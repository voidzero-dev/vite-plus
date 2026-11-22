import { defineConfig } from 'tsdown';

export default defineConfig({
  entry: ['src/library.ts'],
  outDir: 'lib-out',
  dts: false,
});
