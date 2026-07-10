import { defineConfig, lazyPlugins } from 'vite-plus';

export default defineConfig({
  plugins: lazyPlugins(async () => {
    const { default: myLazyPlugin } = await import('./my-plugin');
    return [myLazyPlugin()];
  }),
});
