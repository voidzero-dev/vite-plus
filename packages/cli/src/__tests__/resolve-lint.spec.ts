import { existsSync } from 'node:fs';

import { describe, expect, it } from '@voidzero-dev/vite-plus-test';

import { lint, resolveWindowsTsgolintExecutable } from '../resolve-lint.js';

describe('resolve-lint', () => {
  it('should resolve binPath and OXLINT_TSGOLINT_PATH to existing files', async () => {
    const result = await lint();

    expect(result.binPath).toBeTruthy();
    expect(
      existsSync(result.binPath),
      `oxlint binPath should point to an existing file, got: ${result.binPath}`,
    ).toBe(true);

    const tsgolintPath = result.envs.OXLINT_TSGOLINT_PATH;
    expect(tsgolintPath).toBeTruthy();
    expect(
      existsSync(tsgolintPath),
      `OXLINT_TSGOLINT_PATH should point to an existing file, got: ${tsgolintPath}`,
    ).toBe(true);
  });

  it('should keep the Windows tsgolint executable path absolute', () => {
    const tsgolintPath =
      'C:\\repo\\node_modules\\.pnpm\\vite-plus@0.1.24\\node_modules\\vite-plus\\node_modules\\.bin\\tsgolint.cmd';
    const result = resolveWindowsTsgolintExecutable(
      [
        'C:\\repo\\node_modules\\.pnpm\\vite-plus@0.1.24\\node_modules\\vite-plus\\node_modules\\.bin\\tsgolint.exe',
        tsgolintPath,
      ],
      {
        exists: (path) => path === tsgolintPath,
      },
    );

    expect(result).toBe(tsgolintPath);
    expect(result).not.toMatch(/^\.\\/);
  });
});
