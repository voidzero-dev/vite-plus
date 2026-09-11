import { defineConfig, lazyPlugins } from 'vite-plus';

import mySyncPlugin from './my-plugin';

export default defineConfig({
  plugins: lazyPlugins(() => [mySyncPlugin()]),
});
