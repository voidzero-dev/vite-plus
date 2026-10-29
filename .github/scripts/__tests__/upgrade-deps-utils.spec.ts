/// <reference types="node" />

import { expect, onTestFinished, test, vi } from 'vite-plus/test';

import { findLatestStableVersionForMajor, getLatestNodeVersion } from '../upgrade-deps-utils.ts';

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

test('selects the latest stable Node.js 24 release independently of the LTS tag and index order', async () => {
  onTestFinished(() => {
    vi.unstubAllGlobals();
  });
  const fetchMock = vi.fn().mockResolvedValue(
    Response.json([
      { version: 'v26.13.0', lts: 'Lithium' },
      { version: 'v24.9.0', lts: 'Krypton' },
      { version: 'v24.11.0-rc.1', lts: false },
      { version: 'v24.10.2', lts: 'Krypton' },
      { version: 'v24.10.1', lts: 'Krypton' },
    ]),
  );
  vi.stubGlobal('fetch', fetchMock);
  await expect(getLatestNodeVersion(24)).resolves.toBe('24.10.2');
  expect(fetchMock).toHaveBeenCalledWith('https://nodejs.org/dist/index.json');
});

test.each([
  {
    response: () => new Response('Unavailable', { status: 503, statusText: 'Unavailable' }),
    error: 'Failed to fetch Node.js releases: 503 Unavailable',
  },
  {
    response: () => Response.json({ version: 'v24.10.2' }),
    error: 'Invalid Node.js release index: expected an array',
  },
  {
    response: () => Response.json([{ version: 'v26.13.0' }, { version: 'v24.10.0-rc.1' }]),
    error: 'No stable 24.x version found for Node.js',
  },
])('rejects an unusable Node.js release index: $error', async ({ response, error }) => {
  onTestFinished(() => {
    vi.unstubAllGlobals();
  });
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue(response()));
  await expect(getLatestNodeVersion(24)).rejects.toThrow(error);
});
