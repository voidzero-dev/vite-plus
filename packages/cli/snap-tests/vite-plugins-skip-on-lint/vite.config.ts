import { defineConfig, vitePlugins } from 'vite-plus';

export default defineConfig({
  plugins: [
    vitePlugins(() => {
      throw new Error('Plugins should not be loaded during lint');
    }),
  ],
});
