import { defineConfig, vitePlugins } from 'vite-plus';

import mySyncPlugin from './my-plugin';

export default defineConfig({
  plugins: [vitePlugins(() => [mySyncPlugin()])],
});
