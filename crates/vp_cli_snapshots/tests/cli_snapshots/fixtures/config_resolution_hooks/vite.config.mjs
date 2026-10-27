import { defineConfig, lazyPlugins, normalizePath } from 'vite-plus';

import { pack, staged } from './settings.mjs';

console.log(`config NODE_ENV: ${process.env.NODE_ENV}`);

export default defineConfig({
  // CLI metadata reads retain the command's root override, even when the file sets root.
  root: 'app',
  plugins: [
    lazyPlugins(() => {
      throw new Error('metadata resolution must skip lazy plugin factories');
    }),
    {
      name: 'provide-tool-config',
      apply(config) {
        console.log(`apply root: ${normalizePath(String(config.root))}`);
        return true;
      },
      config: {
        order: 'pre',
        handler(config) {
          console.log(`config root: ${normalizePath(String(config.root))}`);
          return { staged, pack };
        },
      },
      async configResolved(config) {
        console.log(`configResolved root: ${normalizePath(config.root)}`);
        console.log(`configResolved configFile: ${config.configFile}`);
        console.log('configResolved dependencies:');
        for (const dependency of config.configFileDependencies.map(normalizePath).sort()) {
          console.log(`  ${dependency}`);
        }
      },
    },
  ],
});
