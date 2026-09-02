import { describe, expect, it } from 'vitest';

import { runSyncVersionsProtocol, toolchainFromManifest } from '../protocol.ts';

const manifest = {
  schemaVersion: 1,
  nodes: [
    { id: 'vite-plus', version: '0.4.0' },
    { id: 'vitest', version: '5.0.0' },
  ],
};

describe('runSyncVersionsProtocol', () => {
  it('returns only the versioned JSON plan', () => {
    const output = runSyncVersionsProtocol(
      JSON.stringify({
        schemaVersion: 1,
        workspace: '.',
        manifests: [
          {
            path: 'package.json',
            kind: 'packageJson',
            contents: '{"devDependencies":{"vite-plus":"0.3.0"}}\n',
          },
        ],
      }),
      manifest,
    );

    expect(JSON.parse(output)).toEqual({
      schemaVersion: 1,
      tool: { name: 'vite-plus', version: '0.4.0' },
      workspace: '.',
      replacements: [
        {
          path: 'package.json',
          kind: 'packageJson',
          before: '{"devDependencies":{"vite-plus":"0.3.0"}}\n',
          after: '{"devDependencies":{"vite-plus":"0.4.0"}}\n',
        },
      ],
    });
    expect(output.endsWith('\n')).toBe(true);
  });

  it('rejects invalid JSON before planning', () => {
    expect(() => runSyncVersionsProtocol('{', manifest)).toThrow('Invalid sync request JSON');
  });
});

describe('toolchainFromManifest', () => {
  it('reads the exact Vite+ and Vitest versions', () => {
    expect(toolchainFromManifest(manifest)).toEqual({
      vitePlus: '0.4.0',
      vitest: '5.0.0',
    });
  });

  it.each([
    {},
    { schemaVersion: 2, nodes: [] },
    { schemaVersion: 1, nodes: [{ id: 'vite-plus', version: '0.4.0' }] },
    {
      schemaVersion: 1,
      nodes: [
        { id: 'vite-plus', version: '0.4.0' },
        { id: 'vitest', version: '' },
      ],
    },
  ])('rejects an incomplete toolchain manifest', (input) => {
    expect(() => toolchainFromManifest(input)).toThrow();
  });
});
