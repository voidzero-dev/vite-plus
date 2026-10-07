import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

import { describe, expect, test } from 'vitest';

import { OXC_NODE_DIR, OXC_NODE_PATCH, setOxcVersion } from '../patch-oxc-node.ts';

const rootDir = join(import.meta.dirname, '../../../..');
const patch = readFileSync(OXC_NODE_PATCH, 'utf-8');

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

describe('patches/oxc-node.patch', () => {
  test('touches only the crate manifest and source', () => {
    const files = [...patch.matchAll(/^diff --git a\/(\S+) /gm)].map((match) => match[1]);
    expect(files).toEqual(['Cargo.toml', 'src/lib.rs']);
  });

  test('keeps the oxc version out of the patch', () => {
    // `setOxcVersion` owns that line; patch context must not depend on it.
    expect(patch).not.toMatch(/^[+ -]oxc = \{ version/m);
  });

  test('marks every source change', () => {
    expect(patch.match(/^\+.*Vite\+:/gm)?.length).toBeGreaterThanOrEqual(10);
  });

  test.skipIf(!existsSync(join(rootDir, OXC_NODE_DIR, '.git')))(
    'is applied to the vendored checkout',
    () => {
      // `sync-remote` and the CI clone action run `patchOxcNode` before cargo does.
      execFileSync('git', ['apply', '--reverse', '--check', OXC_NODE_PATCH], {
        cwd: join(rootDir, OXC_NODE_DIR),
        stdio: 'pipe',
      });
    },
  );
});
