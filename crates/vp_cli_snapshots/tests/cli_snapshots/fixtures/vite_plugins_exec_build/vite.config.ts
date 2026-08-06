import { defineConfig, lazyPlugins } from 'vite-plus';

export default defineConfig({
  plugins: lazyPlugins(async () => {
    const { default: myPlugin } = await import('./my-plugin');
    return [myPlugin()];
  }),
});
