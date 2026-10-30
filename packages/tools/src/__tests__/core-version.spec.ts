import { describe, expect, test } from 'vitest';

import {
  assertCoreVersionMatchesVite,
  decodeCoreVersion,
  encodeCoreVersion,
  nextCoreVersion,
} from '../core-version.ts';

const PUBLISHED_BEFORE_ALIGNMENT = ['0.1.21-alpha.7', '0.0.2-test.1', '1.0.0-rc.0', '1.4.0'];

describe('encodeCoreVersion', () => {
  test('encodes the Vite patch and the release counter in the patch', () => {
    expect(encodeCoreVersion('8.3.5', 0)).toBe('8.3.500');
    expect(encodeCoreVersion('8.3.5', 1)).toBe('8.3.501');
    expect(encodeCoreVersion('8.4.0', 0)).toBe('8.4.0');
    expect(encodeCoreVersion('8.4.1', 0)).toBe('8.4.100');
    expect(encodeCoreVersion('8.3.5', 99)).toBe('8.3.599');
  });

  test('rejects a Vite prerelease and an out-of-range counter', () => {
    expect(() => encodeCoreVersion('9.0.0-beta.3', 0)).toThrow('Vite prerelease');
    expect(() => encodeCoreVersion('8.3.5', 100)).toThrow('from 0 to 99');
    expect(() => encodeCoreVersion('8.3.5', -1)).toThrow('from 0 to 99');
  });
});

describe('decodeCoreVersion', () => {
  test('decodes stable and prerelease core versions', () => {
    expect(decodeCoreVersion('8.3.500')).toEqual({ vite: '8.3.5', n: 0 });
    expect(decodeCoreVersion('8.3.501')).toEqual({ vite: '8.3.5', n: 1 });
    expect(decodeCoreVersion('8.4.1')).toEqual({ vite: '8.4.0', n: 1 });
    expect(decodeCoreVersion('8.4.0-vp.2')).toEqual({ vite: '8.4.0', n: 0, k: 2 });
    expect(decodeCoreVersion('9.0.0-beta.3.vp.1')).toEqual({ vite: '9.0.0-beta.3', k: 1 });
  });

  test('rejects prereleases without the vp counter', () => {
    expect(() => decodeCoreVersion('0.0.0-commit.0c515e3')).toThrow('not an encoded');
    expect(() => decodeCoreVersion('8.4.0-rc.0')).toThrow('not an encoded');
    expect(() => decodeCoreVersion('8.4.0-vp.x')).toThrow('not an encoded');
    expect(() => decodeCoreVersion('latest')).toThrow('Invalid version');
  });
});

describe('nextCoreVersion', () => {
  test('starts the counter at 0 for a new Vite version', () => {
    expect(
      nextCoreVersion({ vite: '8.3.5', vitePlus: '1.5.0', published: PUBLISHED_BEFORE_ALIGNMENT }),
    ).toBe('8.3.500');
  });

  test('increments the counter for each stable release on one Vite version', () => {
    const published = [...PUBLISHED_BEFORE_ALIGNMENT, '8.3.500', '8.3.501', '8.3.600'];
    expect(nextCoreVersion({ vite: '8.3.5', vitePlus: '1.5.2', published })).toBe('8.3.502');
    expect(nextCoreVersion({ vite: '8.3.6', vitePlus: '1.6.1', published })).toBe('8.3.601');
    expect(nextCoreVersion({ vite: '8.4.0', vitePlus: '1.7.0', published })).toBe('8.4.0');
  });

  test('ignores prereleases and local versions for the stable counter', () => {
    const published = ['8.4.0-vp.0', '8.4.0-vp.1', '8.4.99'];
    expect(nextCoreVersion({ vite: '8.4.0', vitePlus: '1.7.0', published })).toBe('8.4.0');
  });

  test('gives each Vite+ prerelease on a stable Vite a unique vp counter', () => {
    expect(nextCoreVersion({ vite: '8.4.0', vitePlus: '1.7.0-rc.0', published: [] })).toBe(
      '8.4.0-vp.0',
    );
    expect(
      nextCoreVersion({ vite: '8.4.0', vitePlus: '1.8.0-rc.0', published: ['8.4.0-vp.0'] }),
    ).toBe('8.4.0-vp.1');
    expect(
      nextCoreVersion({
        vite: '8.4.0',
        vitePlus: '1.8.0-rc.0',
        published: ['8.4.0', '8.4.0-vp.0', '8.4.0-vp.1'],
      }),
    ).toBe('8.4.1-vp.0');
  });

  test('appends the vp counter to a bundled Vite prerelease', () => {
    expect(nextCoreVersion({ vite: '9.0.0-beta.3', vitePlus: '2.0.0-beta.0', published: [] })).toBe(
      '9.0.0-beta.3.vp.0',
    );
    expect(
      nextCoreVersion({
        vite: '9.0.0-beta.3',
        vitePlus: '2.0.0-beta.1',
        published: ['9.0.0-beta.2.vp.4', '9.0.0-beta.3.vp.0'],
      }),
    ).toBe('9.0.0-beta.3.vp.1');
  });

  test('rejects a stable vite-plus release on a Vite prerelease', () => {
    expect(() =>
      nextCoreVersion({ vite: '9.0.0-beta.3', vitePlus: '2.0.0', published: [] }),
    ).toThrow('cannot bundle the Vite prerelease');
  });

  test('fails when the release counter is exhausted', () => {
    const published = Array.from({ length: 99 }, (_, n) => encodeCoreVersion('8.3.5', n));
    expect(() => nextCoreVersion({ vite: '8.3.5', vitePlus: '1.9.0', published })).toThrow(
      'the maximum',
    );
  });
});

function manifest(version: string, vite = '8.3.5') {
  return { version, bundledVersions: { vite } };
}

describe('assertCoreVersionMatchesVite', () => {
  test('accepts a core version that encodes the bundled Vite', () => {
    expect(() => assertCoreVersionMatchesVite(manifest('8.3.500'), '1.5.0')).not.toThrow();
    expect(() => assertCoreVersionMatchesVite(manifest('8.3.599'), '0.0.0')).not.toThrow();
    expect(() =>
      assertCoreVersionMatchesVite(manifest('8.4.0-vp.1', '8.4.0'), '1.7.0-rc.1'),
    ).not.toThrow();
    expect(() =>
      assertCoreVersionMatchesVite(manifest('9.0.0-beta.3.vp.0', '9.0.0-beta.3'), '2.0.0-beta.0'),
    ).not.toThrow();
  });

  test('rejects a core version that encodes another Vite version', () => {
    expect(() => assertCoreVersionMatchesVite(manifest('8.3.400'), '1.5.0')).toThrow(
      'Set its version to 8.3.500',
    );
    expect(() => assertCoreVersionMatchesVite(manifest('1.4.0'), '1.4.0')).toThrow(
      'encodes Vite 1.4.0',
    );
  });

  test('rejects mixed prerelease and stable versions', () => {
    expect(() => assertCoreVersionMatchesVite(manifest('8.3.500'), '1.5.0-rc.0')).toThrow(
      'both be prereleases',
    );
    expect(() => assertCoreVersionMatchesVite(manifest('8.3.500-vp.0'), '1.5.0')).toThrow(
      'both be prereleases',
    );
  });

  test('rejects the local build counter for a release', () => {
    expect(() =>
      assertCoreVersionMatchesVite(manifest('8.3.599'), '1.5.0', { release: true }),
    ).toThrow('reserved for local builds');
    expect(() =>
      assertCoreVersionMatchesVite(manifest('8.3.598'), '1.5.0', { release: true }),
    ).not.toThrow();
  });
});
