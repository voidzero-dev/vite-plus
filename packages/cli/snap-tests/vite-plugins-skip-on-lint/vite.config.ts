import { defineConfig, vitePlugins } from 'vite-plus';

export default defineConfig({
  plugins: [
    vitePlugins(async () => {
      const { default: heavyPlugin } = await import('./heavy-plugin');
      return [heavyPlugin()];
    }),
  ],
});
