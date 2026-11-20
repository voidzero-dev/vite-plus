import { describe, expect, test } from 'vitest';

import { syncOxcVersions } from '../patch-oxc-node.ts';

describe('syncOxcVersions', () => {
  const cargoToml = `[dependencies]
oxc = { version = "0.153.0", features = [
  "codegen",
] }
oxc_resolver = { version = "11.24.3" }
oxc_sourcemap = "9"
`;
  const workspaceToml = `oxc = { version = "0.159.0" }\noxc_sourcemap = "10.0.0"\n`;

  test('follows the workspace oxc and sourcemap versions and leaves other crates alone', () => {
    const patched = syncOxcVersions(cargoToml, workspaceToml);
    expect(patched).toContain('oxc = { version = "0.159.0", features = [');
    expect(patched).toContain('oxc_sourcemap = "10.0.0"');
    expect(patched).toContain('oxc_resolver = { version = "11.24.3" }');
    expect(syncOxcVersions(patched, workspaceToml)).toBe(patched);
  });

  test('fails without an oxc dependency', () => {
    expect(() => syncOxcVersions('[dependencies]\n', workspaceToml)).toThrow(/`oxc` dependency/);
  });
});
