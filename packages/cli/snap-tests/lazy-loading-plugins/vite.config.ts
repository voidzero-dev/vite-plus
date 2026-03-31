import { defineConfig } from 'vite-plus';

export default defineConfig({
  plugins: [import('./my-plugin').then((m) => m.default())],
});
