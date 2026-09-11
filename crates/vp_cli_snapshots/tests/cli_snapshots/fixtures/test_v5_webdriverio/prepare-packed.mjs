import assert from 'node:assert/strict';
import { existsSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';

// Reuse the browser provisioned for the checkout before installing project peers.
const require = createRequire(import.meta.resolve('vite-plus/package.json'));
const executable = require('playwright').chromium.executablePath();
assert.ok(existsSync(executable), `Chromium is not installed at ${executable}`);
writeFileSync('chromium-path.json', JSON.stringify(executable));
