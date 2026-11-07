import assert from 'node:assert/strict';
import { readdirSync, readFileSync, statSync, utimesSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';

const action = process.argv[2];
if (action === 'stamp') {
  const timestamp = new Date('2020-01-01T00:00:00Z');
  for (const file of ['value.ts', 'legacy.cts']) {
    utimesSync(file, timestamp, timestamp);
  }
} else if (action === 'edit') {
  // vpt has no operation that changes bytes while restoring the exact timestamp.
  for (const file of ['value.ts', 'legacy.cts']) {
    const previous = statSync(file);
    writeFileSync(file, readFileSync(file, 'utf8').replace("'one'", "'two'"));
    utimesSync(file, previous.atime, previous.mtime);
    const current = statSync(file);
    assert.equal(current.size, previous.size);
    assert.equal(current.mtimeMs, previous.mtimeMs);
  }
} else if (action === 'corrupt') {
  const require = createRequire(import.meta.url);
  const packageRoot = path.dirname(require.resolve('vite-plus/package.json'));
  const { oxcNode } = require(path.join(packageRoot, 'binding/index.cjs'));
  // Use the native directory API, so the test follows VpDirs on every platform.
  const root = oxcNode.initializeCache(false, process.versions.node);
  assert.ok(root);
  let records = 0;
  const corrupt = (directory) => {
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      const file = path.join(directory, entry.name);
      if (entry.isDirectory()) corrupt(file);
      else if (/^[a-f0-9]{64}(?:\.pack)?$/.test(entry.name)) {
        writeFileSync(file, 'broken cache record');
        records++;
      }
    }
  };
  corrupt(path.join(root, 'transform'));
  assert.ok(records > 0, 'the priming runs must create transform artifacts');
} else {
  throw new Error(`unknown action: ${action}`);
}
