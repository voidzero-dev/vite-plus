import { defineConfig } from 'vite-plus';

export default defineConfig({
  lazy: async () => {
    const { default: myLazyPlugin } = await import('./my-plugin');
    return { plugins: [myLazyPlugin()] };
  },
});
