import { defineConfig } from 'tsdown';

export default defineConfig({
  entry: 'src/index.ts',
  outDir: 'dist',
  format: ['esm', 'cjs'],
  dts: true,
  bundle: false,
  publicDir: 'public',
  deps: { onlyAllowBundle: false },
});
