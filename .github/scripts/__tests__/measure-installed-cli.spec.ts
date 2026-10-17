import { describe, expect, test } from 'vitest';

import { calculateUnpackedSize, matchesPlatform } from '../measure-installed-cli.mjs';

function packument(size: number, fields: Record<string, unknown> = {}) {
  return { versions: { '1.0.0': { dist: { unpackedSize: size }, ...fields } } };
}

describe('unpacked CLI size', () => {
  test('includes aliased core, deduplicates shared dependencies, and filters native targets', async () => {
    const registry = {
      cli: packument(100, {
        dependencies: { vite: 'npm:@scope/core@1.0.0', shared: '^1.0.0' },
        optionalDependencies: { linux: '1.0.0', musl: '1.0.0', darwin: '1.0.0' },
        devDependencies: { development: '1.0.0' },
        peerDependencies: { peer: '1.0.0' },
      }),
      '@scope/core': packument(200, {
        dependencies: { shared: '^1.0.0', css: '1.0.0' },
        optionalDependencies: { linux: '1.0.0' },
      }),
      shared: {
        versions: {
          '1.0.0': { dist: { unpackedSize: 10 } },
          '1.1.0': { dist: { unpackedSize: 30 } },
        },
      },
      css: packument(40),
      linux: packument(300, { os: ['linux'], cpu: ['x64'], libc: ['glibc'] }),
      musl: packument(400, { os: ['linux'], cpu: ['x64'], libc: ['musl'] }),
      darwin: packument(500, { os: ['darwin'], cpu: ['arm64'] }),
    };
    const load = async (name: keyof typeof registry) => {
      expect(registry[name]).toBeDefined();
      return registry[name];
    };
    const result = await calculateUnpackedSize('cli', '1.0.0', load);
    expect(result.selfSize).toBe(100);
    expect(result.totalSize).toBe(670);
    expect(result.dependencyCount).toBe(4);
    expect(result.dependencies.map((item: { name: string }) => item.name)).toEqual([
      'linux',
      '@scope/core',
      'css',
      'shared',
    ]);
  });

  test('honors excluded platforms and unrestricted packages', () => {
    expect(matchesPlatform({})).toBe(true);
    expect(matchesPlatform({ os: ['!darwin'], cpu: ['!arm64'] })).toBe(true);
    expect(matchesPlatform({ os: ['linux', '!linux'] })).toBe(false);
    expect(matchesPlatform({ cpu: ['arm64'] })).toBe(false);
    expect(matchesPlatform({ libc: ['!glibc'] })).toBe(false);
  });

  test('records missing registry sizes as zero, as npmx does', async () => {
    const registry = {
      cli: packument(100, { dependencies: { legacy: '1.0.0' } }),
      legacy: { versions: { '1.0.0': {} } },
    };
    const result = await calculateUnpackedSize(
      'cli',
      '1.0.0',
      async (name: keyof typeof registry) => registry[name],
    );
    expect(result.totalSize).toBe(100);
    expect(result.dependencies).toEqual([
      { name: 'legacy', version: '1.0.0', size: 0, missingSize: true },
    ]);
  });

  test('fails if a required dependency cannot be resolved', async () => {
    const registry = {
      cli: packument(100, { dependencies: { required: '2.0.0' } }),
      required: packument(200),
    };
    await expect(
      calculateUnpackedSize('cli', '1.0.0', async (name: keyof typeof registry) => registry[name]),
    ).rejects.toThrow('Cannot resolve required@2.0.0');
  });
});
