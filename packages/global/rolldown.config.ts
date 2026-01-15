import { defineConfig } from 'rolldown';

export default defineConfig({
  input: './src/index.ts',
  external: [
    /^node:/,
    // FIXME: Calling `require` for "child_process" in an environment that doesn't expose the `require` function
    'cross-spawn',
    // FIXME: will lost colors if not external
    'picocolors',
  ],
  output: {
    format: 'esm',
    dir: './dist',
    cleanDir: true,
  },
});
