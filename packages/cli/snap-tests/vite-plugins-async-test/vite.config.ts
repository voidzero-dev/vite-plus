import { defineConfig, lazyPlugins } from 'vite-plus';

export default defineConfig({
  plugins: lazyPlugins(async () => {
    const { default: myVitestPlugin } = await import('./my-vitest-plugin');
    return [myVitestPlugin()];
  }),
});
