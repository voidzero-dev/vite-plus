import { defineConfig } from 'tsdown';

export default defineConfig({
  entry: ['src/root-library.ts'],
  outDir: 'custom-out',
  dts: false,
});
