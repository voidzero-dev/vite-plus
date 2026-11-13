import { existsSync } from 'node:fs';

import { describe, expect, it } from 'vitest';

import { lint, resolveWindowsLintBackendExecutable } from '../resolve-lint.js';

describe('resolve-lint', () => {
  it('should resolve binPath and OXLINT_TSGOLINT_PATH to existing files', async () => {
    const result = await lint();

    expect(result.binPath).toBeTruthy();
    expect(
      existsSync(result.binPath),
      `oxlint binPath should point to an existing file, got: ${result.binPath}`,
    ).toBe(true);

    const backendPath = result.envs.OXLINT_TSGOLINT_PATH;
    expect(backendPath).toBeTruthy();
    expect(backendPath).toMatch(/[/\\]tsrs-lint(?:\.(?:cmd|exe))?$/);
    expect(
      existsSync(backendPath),
      `OXLINT_TSGOLINT_PATH should point to an existing file, got: ${backendPath}`,
    ).toBe(true);
  });

  it('should keep the Windows tsrs-lint executable path absolute', () => {
    const backendPath =
      'C:\\repo\\node_modules\\.pnpm\\vite-plus@0.1.24\\node_modules\\vite-plus\\node_modules\\.bin\\tsrs-lint.cmd';
    const result = resolveWindowsLintBackendExecutable(
      [
        'C:\\repo\\node_modules\\.pnpm\\vite-plus@0.1.24\\node_modules\\vite-plus\\node_modules\\.bin\\tsrs-lint.exe',
        backendPath,
      ],
      {
        exists: (path) => path === backendPath,
      },
    );

    expect(result).toBe(backendPath);
    expect(result).not.toMatch(/^\.\\/);
  });
});
