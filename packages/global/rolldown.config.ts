import { defineConfig } from 'rolldown';

export default defineConfig({
  input: './src/index.ts',
  external: [
    /^node:/,
    '@voidzero-dev/vite-plus/bin',
    '@voidzero-dev/vite-plus/binding',
    // FIXME: Calling `require` for "child_process" in an environment that doesn't expose the `require` function
    'cross-spawn',
    // FIXME: will lost colors if not external
    'picocolors',
    // FIXME: Calling `require` for "module" in an environment that doesn't expose the `require` function
    'validate-npm-package-name',
  ],
  output: {
    format: 'esm',
    dir: './dist',
    cleanDir: true,
  },
});
