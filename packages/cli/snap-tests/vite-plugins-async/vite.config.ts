import { defineConfig, vitePlugins } from 'vite-plus';

export default defineConfig({
  plugins: [
    vitePlugins(async () => {
      const { default: myLazyPlugin } = await import('./my-plugin');
      return [myLazyPlugin()];
    }),
  ],
});
