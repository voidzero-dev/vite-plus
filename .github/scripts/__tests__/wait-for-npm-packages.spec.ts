/// <reference types="node" />

import { describe, expect, test, vi } from 'vitest';

import {
  type FetchLike,
  isNpmPackageAvailable,
  parseNpmPackageSpec,
  waitForNpmPackages,
} from '../wait-for-npm-packages.ts';

function response(status: number, body?: unknown) {
  return {
    ok: status >= 200 && status < 300,
    status,
    json: async () => body,
  };
}

describe('isNpmPackageAvailable', () => {
  test('checks the abbreviated packument and its tarball', async () => {
    const fetchImpl = vi
      .fn<FetchLike>()
      .mockResolvedValueOnce(
        response(200, {
          versions: {
            '1.2.3': {
              dist: {
                tarball: 'https://registry.npmjs.org/pkg/-/pkg-1.2.3.tgz',
              },
            },
          },
        }),
      )
      .mockResolvedValueOnce(response(200));

    await expect(
      isNpmPackageAvailable(
        { name: '@scope/pkg', version: '1.2.3' },
        { registry: 'https://registry.npmjs.org/', fetchImpl },
      ),
    ).resolves.toBe(true);

    expect(fetchImpl).toHaveBeenNthCalledWith(1, 'https://registry.npmjs.org/@scope%2fpkg', {
      headers: { accept: 'application/vnd.npm.install-v1+json' },
    });
    expect(fetchImpl).toHaveBeenNthCalledWith(2, 'https://registry.npmjs.org/pkg/-/pkg-1.2.3.tgz', {
      method: 'HEAD',
    });
  });

  test('is unavailable while the version or tarball is missing', async () => {
    const missingVersion = vi.fn<FetchLike>().mockResolvedValue(
      response(200, {
        versions: { '1.2.2': {} },
      }),
    );
    await expect(
      isNpmPackageAvailable(
        { name: 'pkg', version: '1.2.3' },
        { registry: 'https://registry.npmjs.org', fetchImpl: missingVersion },
      ),
    ).resolves.toBe(false);

    const missingTarball = vi
      .fn<FetchLike>()
      .mockResolvedValueOnce(
        response(200, {
          versions: {
            '1.2.3': {
              dist: {
                tarball: 'https://registry.npmjs.org/pkg/-/pkg-1.2.3.tgz',
              },
            },
          },
        }),
      )
      .mockResolvedValueOnce(response(404));
    await expect(
      isNpmPackageAvailable(
        { name: 'pkg', version: '1.2.3' },
        { registry: 'https://registry.npmjs.org', fetchImpl: missingTarball },
      ),
    ).resolves.toBe(false);
  });
});

describe('waitForNpmPackages', () => {
  test('polls until available and always settles after the successful read', async () => {
    let currentTime = 0;
    const sleep = vi.fn(async (milliseconds: number) => {
      currentTime += milliseconds;
    });
    const fetchImpl = vi
      .fn<FetchLike>()
      .mockResolvedValueOnce(response(404))
      .mockResolvedValueOnce(
        response(200, {
          versions: {
            '1.2.3': {
              dist: {
                tarball: 'https://registry.npmjs.org/pkg/-/pkg-1.2.3.tgz',
              },
            },
          },
        }),
      )
      .mockResolvedValueOnce(response(200));

    await waitForNpmPackages([{ name: 'pkg', version: '1.2.3' }], {
      registry: 'https://registry.npmjs.org',
      fetchImpl,
      minSeconds: 60,
      timeoutSeconds: 600,
      pollSeconds: 5,
      sleep,
      now: () => currentTime,
      log: vi.fn(),
    });

    expect(sleep.mock.calls).toEqual([[5_000], [60_000]]);
  });

  test('retries transient read failures until the timeout', async () => {
    let currentTime = 0;
    const fetchImpl = vi.fn<FetchLike>().mockRejectedValue(new Error('temporary failure'));

    await expect(
      waitForNpmPackages([{ name: 'pkg', version: '1.2.3' }], {
        registry: 'https://registry.npmjs.org',
        fetchImpl,
        minSeconds: 0,
        timeoutSeconds: 5,
        pollSeconds: 5,
        sleep: async (milliseconds) => {
          currentTime += milliseconds;
        },
        now: () => currentTime,
        log: vi.fn(),
      }),
    ).rejects.toThrow('Timed out after 5s waiting for npm propagation: pkg@1.2.3');

    expect(fetchImpl).toHaveBeenCalledTimes(2);
  });
});

test('parseNpmPackageSpec supports scoped and unscoped package names', () => {
  expect(parseNpmPackageSpec('@scope/pkg@1.2.3')).toEqual({
    name: '@scope/pkg',
    version: '1.2.3',
  });
  expect(parseNpmPackageSpec('pkg@1.2.3')).toEqual({
    name: 'pkg',
    version: '1.2.3',
  });
  expect(() => parseNpmPackageSpec('@scope/pkg')).toThrow('name@version');
});
