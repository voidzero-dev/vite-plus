import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { describe, expect, test } from 'vite-plus/test';

import { applyTransformCachePatch, setOxcVersion } from '../patch-oxc-node.ts';

describe('setOxcVersion', () => {
  const cargoToml = `[dependencies]
oxc = { version = "0.153.0", features = [
  "codegen",
] }
oxc_resolver = { version = "11.24.3" }
`;

  test('follows the workspace oxc version and leaves other crates alone', () => {
    const patched = setOxcVersion(cargoToml, '0.152.0');
    expect(patched).toContain('oxc = { version = "0.152.0", features = [');
    expect(patched).toContain('oxc_resolver = { version = "11.24.3" }');
    expect(setOxcVersion(patched, '0.152.0')).toBe(patched);
  });

  test('fails without an oxc dependency', () => {
    expect(() => setOxcVersion('[dependencies]\n', '0.152.0')).toThrow(/`oxc` dependency/);
  });
});

describe('applyTransformCachePatch', () => {
  const patch = `--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1 +1 @@
-original
+cached
`;

  test('applies once to an archive checkout and accepts the already-patched state', () => {
    const root = mkdtempSync(join(tmpdir(), 'vp-oxc-cache-patch-'));
    try {
      mkdirSync(join(root, 'oxc-node', 'src'), { recursive: true });
      const source = join(root, 'oxc-node', 'src', 'lib.rs');
      writeFileSync(source, 'original\n');
      expect(applyTransformCachePatch(root, patch)).toBe(true);
      expect(readFileSync(source, 'utf8')).toBe('cached\n');
      expect(applyTransformCachePatch(root, patch)).toBe(false);
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  test('rejects upstream drift without rewriting source', () => {
    const root = mkdtempSync(join(tmpdir(), 'vp-oxc-cache-patch-'));
    try {
      mkdirSync(join(root, 'oxc-node', 'src'), { recursive: true });
      const source = join(root, 'oxc-node', 'src', 'lib.rs');
      writeFileSync(source, 'upstream changed\n');
      expect(() => applyTransformCachePatch(root, patch)).toThrow(/pinned upstream/);
      expect(readFileSync(source, 'utf8')).toBe('upstream changed\n');
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });
});
