import type { Plugin } from 'vite';
import { defineConfig } from 'vite-plus';

const noop = (): Plugin => ({ name: 'noop' });

export default defineConfig({
  plugins: [noop()],
});
