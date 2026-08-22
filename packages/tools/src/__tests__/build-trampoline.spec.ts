import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, test } from 'vitest';

import { resolveCargoArgs, resolveCargoTargetDir } from '../build-trampoline.ts';

const repoRoot = fileURLToPath(new URL('../../../..', import.meta.url));

describe('resolveCargoTargetDir', () => {
  test('resolves relative paths from the repository root', () => {
    expect(resolveCargoTargetDir('artifacts')).toBe(path.join(repoRoot, 'artifacts'));
  });

  test('uses the repository target directory by default', () => {
    expect(resolveCargoTargetDir(undefined)).toBe(path.join(repoRoot, 'target'));
  });

  test('preserves absolute paths', () => {
    const absolute = path.resolve(repoRoot, 'custom-artifacts');
    expect(resolveCargoTargetDir(absolute)).toBe(absolute);
  });
});

describe('resolveCargoArgs', () => {
  test('uses cargo build by default', () => {
    expect(resolveCargoArgs(['--release', '--target', 'x86_64-pc-windows-msvc'])).toEqual([
      'build',
      '--release',
      '--target',
      'x86_64-pc-windows-msvc',
    ]);
  });

  test('uses cargo xwin build when requested', () => {
    expect(resolveCargoArgs(['--xwin', '--release', '--target', 'x86_64-pc-windows-msvc'])).toEqual(
      ['xwin', 'build', '--release', '--target', 'x86_64-pc-windows-msvc'],
    );
  });
});
