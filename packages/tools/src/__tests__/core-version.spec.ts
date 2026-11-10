import { describe, expect, test } from 'vitest';

import {
  assertCoreVersionMatchesVite,
  assertCoreVersionUnclaimed,
  decodeCoreVersion,
  encodeCoreVersion,
  initialCoreVersion,
  nextCoreVersion,
} from '../core-version.ts';

const PUBLISHED_BEFORE_ALIGNMENT = ['0.1.21-alpha.7', '0.0.2-test.1', '1.0.0-rc.0', '1.4.0'];

describe('encodeCoreVersion', () => {
  test('encodes the Vite patch and the release counter in the patch', () => {
    expect(encodeCoreVersion('8.3.5', 0)).toBe('8.3.5000');
    expect(encodeCoreVersion('8.3.5', 1)).toBe('8.3.5001');
    expect(encodeCoreVersion('8.4.0', 0)).toBe('8.4.0');
    expect(encodeCoreVersion('8.4.1', 0)).toBe('8.4.1000');
    expect(encodeCoreVersion('8.3.5', 999)).toBe('8.3.5999');
  });

  test('rejects a Vite prerelease and an out-of-range counter', () => {
    expect(() => encodeCoreVersion('9.0.0-beta.3', 0)).toThrow('Vite prerelease');
    expect(() => encodeCoreVersion('8.3.5', 1000)).toThrow('from 0 to 999');
    expect(() => encodeCoreVersion('8.3.5', -1)).toThrow('from 0 to 999');
  });
});

describe('decodeCoreVersion', () => {
  test('decodes stable and prerelease core versions', () => {
    expect(decodeCoreVersion('8.3.5000')).toEqual({ vite: '8.3.5', n: 0 });
    expect(decodeCoreVersion('8.3.5001')).toEqual({ vite: '8.3.5', n: 1 });
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

// Each release publishes its core version before the next one is prepared.
function releaseSequence(vite: string, vitePlusVersions: string[]): string[] {
  const published: string[] = [];
  for (const vitePlus of vitePlusVersions) {
    published.push(nextCoreVersion({ vite, vitePlus, published }));
  }
  return published;
}

describe('nextCoreVersion', () => {
  test('starts the counter at 0 for a new Vite version', () => {
    expect(
      nextCoreVersion({ vite: '8.3.5', vitePlus: '1.5.0', published: PUBLISHED_BEFORE_ALIGNMENT }),
    ).toBe('8.3.5000');
  });

  test('increments the counter for each stable release on one Vite version', () => {
    const published = [...PUBLISHED_BEFORE_ALIGNMENT, '8.3.5000', '8.3.5001', '8.3.6000'];
    expect(nextCoreVersion({ vite: '8.3.5', vitePlus: '1.5.2', published })).toBe('8.3.5002');
    expect(nextCoreVersion({ vite: '8.3.6', vitePlus: '1.6.1', published })).toBe('8.3.6001');
    expect(nextCoreVersion({ vite: '8.4.0', vitePlus: '1.7.0', published })).toBe('8.4.0');
  });

  test('ignores prereleases and local versions for the stable counter', () => {
    const published = ['8.4.0-vp.0', '8.4.0-vp.1', '8.4.999'];
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
    const published = Array.from({ length: 999 }, (_, n) => encodeCoreVersion('8.3.5', n));
    expect(() => nextCoreVersion({ vite: '8.3.5', vitePlus: '1.9.0', published })).toThrow(
      'the maximum',
    );
  });

  test('changes only the counter for minor and major releases on one Vite version', () => {
    expect(releaseSequence('8.3.7', ['1.6.0', '1.7.0', '1.8.0', '2.0.0'])).toEqual([
      '8.3.7000',
      '8.3.7001',
      '8.3.7002',
      '8.3.7003',
    ]);
  });

  test('gives betas on a stable Vite the base of the next stable release', () => {
    expect(
      releaseSequence('8.3.7', ['1.6.0', '1.7.0-beta.0', '1.7.0-beta.1', '1.7.0', '1.8.0-beta.0']),
    ).toEqual(['8.3.7000', '8.3.7001-vp.0', '8.3.7001-vp.1', '8.3.7001', '8.3.7002-vp.0']);
  });
});

describe('initialCoreVersion', () => {
  test('starts the counter for a stable and a prerelease Vite', () => {
    expect(initialCoreVersion('8.4.2')).toBe('8.4.2000');
    expect(initialCoreVersion('9.0.0-beta.3')).toBe('9.0.0-beta.3.vp.0');
  });
});

function manifest(version: string, vite = '8.3.5', vitePlusVersion?: string) {
  return { version, bundledVersions: { vite }, vitePlusVersion };
}

describe('assertCoreVersionMatchesVite', () => {
  test('accepts a core version that encodes the bundled Vite', () => {
    expect(() => assertCoreVersionMatchesVite(manifest('8.3.5000'), '1.5.0')).not.toThrow();
    expect(() => assertCoreVersionMatchesVite(manifest('8.3.5999'), '0.0.0')).not.toThrow();
    expect(() =>
      assertCoreVersionMatchesVite(manifest('8.4.0-vp.1', '8.4.0'), '1.7.0-rc.1'),
    ).not.toThrow();
    expect(() =>
      assertCoreVersionMatchesVite(manifest('9.0.0-beta.3.vp.0', '9.0.0-beta.3'), '2.0.0-beta.0'),
    ).not.toThrow();
  });

  test('rejects a core version that encodes another Vite version', () => {
    expect(() => assertCoreVersionMatchesVite(manifest('8.3.4000'), '1.5.0')).toThrow(
      'Set its version to 8.3.5000',
    );
    expect(() => assertCoreVersionMatchesVite(manifest('1.4.0'), '1.4.0')).toThrow(
      'encodes Vite 1.4.0',
    );
  });

  test('allows a Vite prerelease with a stable vite-plus between releases', () => {
    expect(() =>
      assertCoreVersionMatchesVite(manifest('9.0.0-beta.3.vp.0', '9.0.0-beta.3'), '1.9.0'),
    ).not.toThrow();
  });

  test('rejects mixed prerelease and stable versions for a release', () => {
    expect(() =>
      assertCoreVersionMatchesVite(manifest('8.3.5000', '8.3.5', '1.5.0-rc.0'), '1.5.0-rc.0', {
        release: true,
      }),
    ).toThrow('both be prereleases');
    expect(() =>
      assertCoreVersionMatchesVite(manifest('8.3.5000-vp.0', '8.3.5', '1.5.0'), '1.5.0', {
        release: true,
      }),
    ).toThrow('both be prereleases');
  });

  test('rejects the local build counter for a release', () => {
    expect(() =>
      assertCoreVersionMatchesVite(manifest('8.3.5999', '8.3.5', '1.5.0'), '1.5.0', {
        release: true,
      }),
    ).toThrow('reserved for local builds');
    expect(() =>
      assertCoreVersionMatchesVite(manifest('8.3.5998', '8.3.5', '1.5.0'), '1.5.0', {
        release: true,
      }),
    ).not.toThrow();
  });

  test('requires the manifest to record the released vite-plus version', () => {
    expect(() =>
      assertCoreVersionMatchesVite(manifest('8.3.5000'), '1.5.0', { release: true }),
    ).toThrow('records vite-plus (none)');
    expect(() =>
      assertCoreVersionMatchesVite(manifest('8.3.5000', '8.3.5', '1.4.0'), '1.5.0', {
        release: true,
      }),
    ).toThrow('records vite-plus 1.4.0, but the release is vite-plus 1.5.0');
  });
});

describe('assertCoreVersionUnclaimed', () => {
  test('accepts an unpublished core version and a retry of the same release', () => {
    expect(() => assertCoreVersionUnclaimed(undefined, '8.3.5001', '1.5.1')).not.toThrow();
    expect(() =>
      assertCoreVersionUnclaimed({ vitePlusVersion: '1.5.1' }, '8.3.5001', '1.5.1'),
    ).not.toThrow();
  });

  test('rejects a core version that another release published', () => {
    expect(() =>
      assertCoreVersionUnclaimed({ vitePlusVersion: '1.5.1' }, '8.3.5001', '1.6.0'),
    ).toThrow('already published for vite-plus 1.5.1');
    expect(() => assertCoreVersionUnclaimed({}, '8.3.5000', '1.6.0')).toThrow(
      'already published for vite-plus (unknown)',
    );
  });
});
