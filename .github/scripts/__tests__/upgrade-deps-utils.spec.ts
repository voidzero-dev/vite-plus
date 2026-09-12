/// <reference types="node" />

import { expect, test } from 'vitest';

import { findLatestStableVersionForMajor } from '../upgrade-deps-utils.ts';

test('selects the highest stable version from the supported major', () => {
  expect(
    findLatestStableVersionForMajor(
      ['3.2.4', '4.1.11', '5.0.0', '4.10.0', '4.11.0-beta.1', '4.2.12'],
      4,
    ),
  ).toBe('4.10.0');
});

test('returns undefined when the supported major has no stable release', () => {
  expect(findLatestStableVersionForMajor(['4.2.0-beta.1', '5.0.0'], 4)).toBeUndefined();
});

test('keeps Vitest upgrades on v5 after the migration', () => {
  expect(
    findLatestStableVersionForMajor(['4.1.11', '5.0.0', '5.1.0', '5.2.0-beta.1', '6.0.0'], 5),
  ).toBe('5.1.0');
});
