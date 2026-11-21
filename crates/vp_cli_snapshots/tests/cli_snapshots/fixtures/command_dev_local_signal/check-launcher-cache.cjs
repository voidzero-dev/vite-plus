const assert = require('node:assert/strict');
const { readdirSync, writeFileSync } = require('node:fs');
const { getCompileCacheDir } = require('node:module');
const { basename } = require('node:path');

if (basename(process.argv[1] ?? '') === 'vite.js') {
  const directory = getCompileCacheDir();
  assert.ok(directory, 'Node enabled the compile cache for Vite');
  assert.ok(
    readdirSync(directory, { recursive: true, withFileTypes: true }).some((entry) => entry.isFile()),
    'vp saves its compile cache before starting Vite',
  );
  writeFileSync(process.env.VP_PROBE_CACHE_PROOF, '');
}
