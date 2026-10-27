import assert from 'node:assert/strict';
import { realpathSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { defineConfig, normalizePath } from 'vite-plus';

const configFile = fileURLToPath(import.meta.url);
const workspaceDir = dirname(configFile);
const packageDir = join(workspaceDir, 'packages', 'lib');

function assertPackageRoot(root) {
  assert.equal(realpathSync.native(root), realpathSync.native(packageDir));
  assert.equal(realpathSync.native(process.cwd()), realpathSync.native(packageDir));
}

export default defineConfig({
  // The existing CLI contract overrides this with the package cwd when reading pack settings.
  root: 'configured-root',
  plugins: [
    {
      name: 'verify-shared-config-package-root',
      apply(config) {
        assertPackageRoot(config.root);
        return true;
      },
      config: {
        order: 'pre',
        handler(config) {
          assertPackageRoot(config.root);
        },
      },
      configResolved(config) {
        assertPackageRoot(config.root);
        assert.equal(config.configFile, normalizePath(configFile));
        writeFileSync('resolved.txt', 'shared config hooks used package cwd\n');
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
