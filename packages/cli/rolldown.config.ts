import { defineConfig } from 'rolldown';

export default defineConfig({
  input: {
    create: './src/create/bin.ts',
    migrate: './src/migration/bin.ts',
    version: './src/version.ts',
  },
  treeshake: false,
  external(source) {
    if (source.startsWith('node:')) {
      return true;
    }
    if (source === 'cross-spawn' || source === 'picocolors') {
      return true;
    }
    if (source === '../../binding/index.js' || source === '../binding/index.js') {
      return true;
    }
    return false;
  },
  plugins: [
    {
      name: 'fix-binding-path',
      // Rewrite the binding import path for the output directory.
      // Source files import from ../../binding/index.js (relative to src/*/).
      // Output is in dist/global/, so the correct path is ../../binding/index.js (two dirs up).
      // Rolldown normalizes it to ../binding/index.js which is wrong.
      renderChunk(code) {
        if (code.includes('../binding/index.js')) {
          return { code: code.replaceAll('../binding/index.js', '../../binding/index.js') };
        }
        return null;
      },
    },
  ],
  output: {
    format: 'esm',
    dir: './dist/global',
    cleanDir: true,
  },
});
