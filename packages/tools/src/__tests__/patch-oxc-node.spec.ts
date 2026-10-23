import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';

import { afterEach, describe, expect, test } from 'vitest';

import { applySourcePatch, setOxcVersion } from '../patch-oxc-node.ts';

const temporaryDirs: string[] = [];
afterEach(() => {
  for (const dir of temporaryDirs.splice(0)) {
    rmSync(dir, { recursive: true, force: true });
  }
});

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

function fixture(contents = 'before\n') {
  const root = mkdtempSync(path.join(tmpdir(), 'vp-oxc-patch-'));
  temporaryDirs.push(root);
  const sourceDir = path.join(root, 'source with spaces');
  mkdirSync(sourceDir);
  const source = path.join(sourceDir, 'lib.rs');
  const patch = path.join(root, 'loader context.patch');
  writeFileSync(source, contents);
  writeFileSync(
    patch,
    'diff --git a/lib.rs b/lib.rs\n--- a/lib.rs\n+++ b/lib.rs\n@@ -1 +1 @@\n-before\n+after\n',
  );
  return { sourceDir, source, patch };
}

describe('applySourcePatch', () => {
  test('applies the patch and accepts the same checkout on repeated setup', () => {
    const { sourceDir, source, patch } = fixture();
    applySourcePatch(sourceDir, patch);
    expect(readFileSync(source, 'utf8')).toBe('after\n');
    applySourcePatch(sourceDir, patch);
    expect(readFileSync(source, 'utf8')).toBe('after\n');
  });

  test('rejects upstream drift without changing the source', () => {
    const { sourceDir, source, patch } = fixture('upstream changed\n');
    expect(() => applySourcePatch(sourceDir, patch)).toThrow(/does not match the upstream source/);
    expect(readFileSync(source, 'utf8')).toBe('upstream changed\n');
  });

  test('does not accept or partially apply an incomplete upstream patch', () => {
    const { sourceDir, source, patch } = fixture();
    const second = path.join(sourceDir, 'other.rs');
    writeFileSync(second, 'different\n');
    writeFileSync(
      patch,
      readFileSync(patch, 'utf8') +
        'diff --git a/other.rs b/other.rs\n--- a/other.rs\n+++ b/other.rs\n@@ -1 +1 @@\n-expected\n+patched\n',
    );
    expect(() => applySourcePatch(sourceDir, patch)).toThrow(/does not match the upstream source/);
    expect(readFileSync(source, 'utf8')).toBe('before\n');
    expect(readFileSync(second, 'utf8')).toBe('different\n');
  });
});
