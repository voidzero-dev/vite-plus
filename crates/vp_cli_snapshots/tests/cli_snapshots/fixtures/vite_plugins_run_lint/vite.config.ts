import { defineConfig, lazyPlugins } from 'vite-plus';

export default defineConfig({
  plugins: lazyPlugins(async () => {
    const { default: heavyPlugin } = await import('./heavy-plugin');
    return [heavyPlugin()];
  }),
});
