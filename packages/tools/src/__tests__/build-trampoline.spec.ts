import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, test } from 'vitest';

import { resolveCargoTargetDir } from '../build-trampoline.ts';

const repoRoot = fileURLToPath(new URL('../../../..', import.meta.url));

describe('resolveCargoTargetDir', () => {
  test('anchors relative paths to the repository root', () => {
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
