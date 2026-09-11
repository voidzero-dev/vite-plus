import assert from 'node:assert/strict';
import { existsSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';

// Reuse the checkout's browser binary, but install all JS packages from tarballs.
const require = createRequire(import.meta.resolve('vite-plus/package.json'));
const executable = require('playwright').chromium.executablePath();
assert.ok(existsSync(executable), `Chromium is not installed at ${executable}`);
writeFileSync('chromium-path.json', JSON.stringify(executable));

// Follow the selected runner so this regression survives the next Vitest upgrade
// and removal of the temporary backport, without installing a second Vitest copy.
const version = require('vitest/package.json').version;
writeFileSync('package.json', JSON.stringify({
  name: 'test-v5-browser-defines',
  private: true,
  type: 'module',
  packageManager: 'pnpm@11.24.0',
  devDependencies: {
    'vite-plus': 'latest',
    vitest: version,
    '@vitest/browser-playwright': version,
  },
}, null, 2));
