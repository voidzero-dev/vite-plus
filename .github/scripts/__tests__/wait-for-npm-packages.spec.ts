/// <reference types="node" />

import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';

import {
  type FetchLike,
  type WaitForNpmPackagesOptions,
  isNpmPackageAvailable,
  parseNpmPackageSpec,
  waitForNpmPackages,
} from '../wait-for-npm-packages.ts';
import { response, stalledFetch } from './npm-registry.ts';

const pkg = { name: 'pkg', version: '1.2.3' };
const tarball = 'https://registry.npmjs.org/pkg/-/pkg-1.2.3.tgz';
const packument = { versions: { '1.2.3': { dist: { tarball } } } };
const pendingPackages = [
  { name: 'first', version: '1.2.3' },
  { name: 'second', version: '1.2.3' },
];

function options(
  fetchImpl: FetchLike,
  overrides: Partial<WaitForNpmPackagesOptions> = {},
): WaitForNpmPackagesOptions {
  return {
    registry: 'https://registry.npmjs.org',
    fetchImpl,
    minSeconds: 0,
    timeoutSeconds: 5,
    pollSeconds: 1,
    sleep: vi.fn(
      (milliseconds) => new Promise<void>((resolve) => setTimeout(resolve, milliseconds)),
    ),
    now: Date.now,
    log: vi.fn(),
    ...overrides,
  };
}

describe('isNpmPackageAvailable', () => {
  test('checks the abbreviated packument and its tarball', async () => {
    const fetchImpl = vi
      .fn<FetchLike>()
      .mockResolvedValueOnce(response(200, packument))
      .mockResolvedValueOnce(response(200));

    await expect(
      isNpmPackageAvailable(
        { ...pkg, name: '@scope/pkg' },
        { registry: 'https://registry.npmjs.org/', fetchImpl },
      ),
    ).resolves.toBe(true);

    expect(fetchImpl).toHaveBeenNthCalledWith(1, 'https://registry.npmjs.org/@scope%2fpkg', {
      headers: { accept: 'application/vnd.npm.install-v1+json' },
      signal: undefined,
    });
    expect(fetchImpl).toHaveBeenNthCalledWith(2, tarball, {
      method: 'HEAD',
      signal: undefined,
    });
  });

  test('is unavailable while the version or tarball is missing', async () => {
    const missingVersion = vi
      .fn<FetchLike>()
      .mockResolvedValue(response(200, { versions: { '1.2.2': {} } }));
    await expect(isNpmPackageAvailable(pkg, options(missingVersion))).resolves.toBe(false);

    const missingTarball = vi
      .fn<FetchLike>()
      .mockResolvedValueOnce(response(200, packument))
      .mockResolvedValueOnce(response(404));
    await expect(isNpmPackageAvailable(pkg, options(missingTarball))).resolves.toBe(false);
  });
});

describe('waitForNpmPackages', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  test('polls until available and always settles after the successful read', async () => {
    const fetchImpl = vi
      .fn<FetchLike>()
      .mockResolvedValueOnce(response(404))
      .mockResolvedValueOnce(response(200, packument))
      .mockResolvedValueOnce(response(200));
    const waitOptions = options(fetchImpl, { minSeconds: 60, timeoutSeconds: 600, pollSeconds: 5 });
    const result = waitForNpmPackages([pkg], waitOptions);

    await vi.advanceTimersByTimeAsync(65_000);
    await result;

    expect(waitOptions.sleep).toHaveBeenCalledTimes(2);
    expect(waitOptions.sleep).toHaveBeenNthCalledWith(1, 5_000);
    expect(waitOptions.sleep).toHaveBeenNthCalledWith(2, 60_000);
    expect(vi.getTimerCount()).toBe(0);
  });

  test('retries transient read failures until the timeout', async () => {
    const fetchImpl = vi.fn<FetchLike>().mockRejectedValue(new Error('temporary failure'));
    const waitOptions = options(fetchImpl, { pollSeconds: 2 });
    const result = expect(waitForNpmPackages([pkg], waitOptions)).rejects.toThrow(
      'Timed out after 5s waiting for npm propagation: pkg@1.2.3',
    );

    await vi.advanceTimersByTimeAsync(5_000);
    await result;

    expect(fetchImpl).toHaveBeenCalledTimes(3);
    expect(waitOptions.sleep).toHaveBeenLastCalledWith(1_000);
    expect(vi.getTimerCount()).toBe(0);
  });

  test('checks the deadline before each package', async () => {
    const fetchImpl = vi.fn<FetchLike>(async () => {
      vi.setSystemTime(Date.now() + 5_000);
      return response(404);
    });

    await expect(waitForNpmPackages(pendingPackages, options(fetchImpl))).rejects.toThrow(
      'Timed out after 5s waiting for npm propagation: first@1.2.3, second@1.2.3',
    );

    expect(fetchImpl).toHaveBeenCalledTimes(1);
    expect(vi.getTimerCount()).toBe(0);
  });

  test('aborts a stalled request at the deadline and skips later packages', async () => {
    const fetchImpl = vi.fn(stalledFetch);
    const result = expect(waitForNpmPackages(pendingPackages, options(fetchImpl))).rejects.toThrow(
      'Timed out after 5s waiting for npm propagation: first@1.2.3, second@1.2.3',
    );

    await vi.advanceTimersByTimeAsync(5_000);
    await result;

    expect(fetchImpl).toHaveBeenCalledTimes(1);
    expect(fetchImpl.mock.calls[0]?.[1]?.signal?.aborted).toBe(true);
    expect(vi.getTimerCount()).toBe(0);
  });
});

test('parseNpmPackageSpec supports scoped and unscoped package names', () => {
  expect(parseNpmPackageSpec('@scope/pkg@1.2.3')).toEqual({
    name: '@scope/pkg',
    version: '1.2.3',
  });
  expect(parseNpmPackageSpec('pkg@1.2.3')).toEqual(pkg);
  expect(() => parseNpmPackageSpec('@scope/pkg')).toThrow('name@version');
});
