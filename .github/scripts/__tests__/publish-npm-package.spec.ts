/// <reference types="node" />

import { describe, expect, test, vi } from 'vitest';

import {
  type PublishCommandRunner,
  type PublishNpmPackageOptions,
  isAlreadyPublishedError,
  publishNpmPackage,
} from '../publish-npm-package.ts';
import type { FetchLike } from '../wait-for-npm-packages.ts';
import { response, stalledFetch } from './npm-registry.ts';

const pkg = { name: '@scope/pkg', version: '1.2.3' };

function options(fetchImpl: FetchLike, runCommand: PublishCommandRunner): PublishNpmPackageOptions {
  return {
    pkg,
    command: 'npm',
    args: ['publish'],
    cwd: '/workspace/pkg',
    registry: 'https://registry.npmjs.org',
    fetchImpl,
    runCommand,
    log: vi.fn(),
    warn: vi.fn(),
  };
}

describe('publishNpmPackage', () => {
  test('skips an exact version that is already visible', async () => {
    const fetchImpl = vi.fn<FetchLike>().mockResolvedValue(
      response(200, {
        versions: { '1.2.3': {} },
      }),
    );
    const runCommand = vi.fn<PublishCommandRunner>();

    await expect(publishNpmPackage(options(fetchImpl, runCommand))).resolves.toBe(
      'already-published',
    );
    expect(runCommand).not.toHaveBeenCalled();
  });

  test('publishes a version that is not visible', async () => {
    const fetchImpl = vi.fn<FetchLike>().mockResolvedValue(response(404));
    const runCommand = vi.fn<PublishCommandRunner>().mockResolvedValue({
      exitCode: 0,
      output: 'published',
    });

    await expect(publishNpmPackage(options(fetchImpl, runCommand))).resolves.toBe('published');
    expect(runCommand).toHaveBeenCalledWith('npm', ['publish'], '/workspace/pkg');
  });

  test('recovers when npm accepted a version that scanning still hides', async () => {
    const fetchImpl = vi.fn<FetchLike>().mockResolvedValue(response(404));
    const runCommand = vi.fn<PublishCommandRunner>().mockResolvedValue({
      exitCode: 1,
      output:
        'npm error 403 Forbidden - You cannot publish over the previously published versions: 1.2.3.',
    });

    await expect(publishNpmPackage(options(fetchImpl, runCommand))).resolves.toBe(
      'already-published',
    );
  });

  test('does not swallow unrelated publish failures', async () => {
    const fetchImpl = vi.fn<FetchLike>().mockResolvedValue(response(404));
    const runCommand = vi.fn<PublishCommandRunner>().mockResolvedValue({
      exitCode: 1,
      output: 'npm error 403 Authentication failed',
    });

    await expect(publishNpmPackage(options(fetchImpl, runCommand))).rejects.toThrow(
      'Failed to publish @scope/pkg@1.2.3: exit code 1',
    );
  });

  test('publishes after a transient preflight read failure', async () => {
    const fetchImpl = vi.fn<FetchLike>().mockRejectedValue(new Error('registry unavailable'));
    const runCommand = vi.fn<PublishCommandRunner>().mockResolvedValue({
      exitCode: 0,
      output: 'published',
    });
    const publishOptions = options(fetchImpl, runCommand);

    await expect(publishNpmPackage(publishOptions)).resolves.toBe('published');
    expect(publishOptions.warn).toHaveBeenCalledOnce();
  });

  test('publishes after a stalled preflight read times out', async ({ onTestFinished }) => {
    vi.useFakeTimers();
    onTestFinished(() => {
      vi.useRealTimers();
    });
    const fetchImpl = vi.fn(stalledFetch);
    const runCommand = vi.fn<PublishCommandRunner>().mockResolvedValue({
      exitCode: 0,
      output: 'published',
    });
    const publishOptions = options(fetchImpl, runCommand);
    const result = publishNpmPackage(publishOptions);

    await vi.advanceTimersByTimeAsync(9_999);
    expect(runCommand).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(1);

    await expect(result).resolves.toBe('published');
    expect(publishOptions.warn).toHaveBeenCalledOnce();
    expect(runCommand).toHaveBeenCalledWith('npm', ['publish'], '/workspace/pkg');
    expect(fetchImpl.mock.calls[0]?.[1]?.signal?.aborted).toBe(true);
    expect(vi.getTimerCount()).toBe(0);
  });

  const releasePackages = [
    '@scope/native-linux',
    '@scope/native-win32',
    '@scope/prompts',
    '@scope/core',
    'vite-plus',
    '@scope/global-linux',
    '@scope/global-win32',
    'create-vite-plus',
  ];
  test.each(
    releasePackages.flatMap((name) => [
      { name, accepted: false },
      { name, accepted: true },
    ]),
  )('resumes publication after $name fails (accepted=$accepted)', async ({ name, accepted }) => {
    const published = new Set<string>();
    const uploads: string[] = [];
    let interrupted = false;
    async function publishRelease() {
      for (const packageName of releasePackages) {
        const fetchImpl: FetchLike = async () =>
          response(200, {
            // An accepted upload can remain hidden during npm's scanning window.
            versions:
              published.has(packageName) && !(accepted && packageName === name)
                ? { [pkg.version]: {} }
                : {},
          });
        const runCommand: PublishCommandRunner = async () => {
          uploads.push(packageName);
          if (packageName === name && !interrupted) {
            interrupted = true;
            if (accepted) {
              published.add(packageName);
            }
            return { exitCode: 1, output: 'connection lost' };
          }
          if (published.has(packageName)) {
            return {
              exitCode: 1,
              output: 'You cannot publish over the previously published versions',
            };
          }
          published.add(packageName);
          return { exitCode: 0, output: 'published' };
        };
        await publishNpmPackage({
          ...options(fetchImpl, runCommand),
          pkg: { name: packageName, version: pkg.version },
        });
      }
    }
    await expect(publishRelease()).rejects.toThrow(`Failed to publish ${name}@${pkg.version}`);
    expect(published.size).toBe(releasePackages.indexOf(name) + Number(accepted));
    await publishRelease();
    expect(published).toEqual(new Set(releasePackages));
    for (const packageName of releasePackages) {
      expect(uploads.filter((upload) => upload === packageName)).toHaveLength(
        packageName === name ? 2 : 1,
      );
    }
  });
});

test('recognizes npm immutable-version errors only', () => {
  expect(isAlreadyPublishedError('npm ERR! code EPUBLISHCONFLICT')).toBe(true);
  expect(isAlreadyPublishedError('npm error 403 Authentication failed')).toBe(false);
});
