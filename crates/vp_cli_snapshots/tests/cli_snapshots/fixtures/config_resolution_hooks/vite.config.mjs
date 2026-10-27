import assert from 'node:assert/strict';
import { realpathSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { defineConfig, lazyPlugins, normalizePath } from 'vite-plus';

import { pack, staged } from './settings.mjs';

const configFile = fileURLToPath(import.meta.url);
const configDir = dirname(configFile);
const root = join(configDir, 'app');

assert.equal(process.env.NODE_ENV, 'development');

export default defineConfig({
  root: 'app',
  plugins: [
    lazyPlugins(() => {
      throw new Error('metadata resolution must skip lazy plugin factories');
    }),
    {
      name: 'provide-tool-config',
      apply(config) {
        assert.equal(config.root, root);
        return true;
      },
      config: {
        order: 'pre',
        handler(config) {
          assert.equal(config.root, root);
          return { staged, pack };
        },
      },
      async configResolved(config) {
        assert.equal(config.configFile, normalizePath(configFile));
        assert.equal(realpathSync.native(config.root), realpathSync.native(root));
        assert.ok(
          config.configFileDependencies.includes(normalizePath(join(configDir, 'settings.mjs'))),
        );
        writeFileSync(join(configDir, 'resolved.txt'), 'configResolved passed\n');
      },
    },
  ],
});
