import { outDir } from './options.ts';

export const viteOutDir = 'dist';

export default {
  entry: ['src/lib.ts'],
  outDir,
  publicDir: 'public',
};
