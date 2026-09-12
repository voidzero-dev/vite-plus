import { defineConfig } from 'vite-plus/config';

export default defineConfig({
  build: { sourcemap: true },
  fmt: { semi: true },
  lint: { rules: { 'no-debugger': 'error' } },
  test: { include: ['tests/**/*.test.ts'] },
  run: { tasks: { verify: { command: 'node verify.mjs' } } },
});

throw new Error('Static tasks must not evaluate the Vite config.');
