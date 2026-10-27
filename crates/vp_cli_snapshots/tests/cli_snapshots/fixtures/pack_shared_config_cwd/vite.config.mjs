import { defineConfig, normalizePath } from 'vite-plus';

function printRoot(hook, config) {
  console.log(`${hook} root: ${normalizePath(String(config.root))}`);
  console.log(`${hook} cwd: ${normalizePath(process.cwd())}`);
}

export default defineConfig({
  // The existing CLI contract overrides this with the package cwd when reading pack settings.
  root: 'configured-root',
  plugins: [
    {
      name: 'print-shared-config-package-root',
      apply(config) {
        printRoot('apply', config);
        return true;
      },
      config: {
        order: 'pre',
        handler(config) {
          printRoot('config', config);
        },
      },
      configResolved(config) {
        printRoot('configResolved', config);
        console.log(`configResolved configFile: ${config.configFile}`);
      },
    },
  ],
  pack: {
    entry: ['src/index.js'],
    outDir: 'dist',
    dts: false,
    format: 'esm',
  },
});
