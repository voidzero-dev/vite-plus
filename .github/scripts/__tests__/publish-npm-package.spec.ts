/// <reference types="node" />

import { describe, expect, test, vi } from 'vitest';

import {
  type PublishCommandRunner,
  type PublishNpmPackageOptions,
  isAlreadyPublishedError,
  publishNpmPackage,
} from '../publish-npm-package.ts';
import type { FetchLike } from '../wait-for-npm-packages.ts';

const pkg = { name: '@scope/pkg', version: '1.2.3' };

function response(status: number, body: unknown = undefined) {
  return {
    ok: status >= 200 && status < 300,
    status,
    json: async () => body,
  };
}

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
});

test('recognizes npm immutable-version errors only', () => {
  expect(isAlreadyPublishedError('npm ERR! code EPUBLISHCONFLICT')).toBe(true);
  expect(isAlreadyPublishedError('npm error 403 Authentication failed')).toBe(false);
});
