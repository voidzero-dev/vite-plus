import { existsSync, mkdirSync, mkdtempSync, realpathSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { describe, expect, it } from 'vitest';

import { lint, resolveWindowsTsgolintExecutable } from '../resolve-lint.js';

describe('resolve-lint', () => {
  it('should not run an oxlint installed in a parent directory', async () => {
    const originalCwd = process.cwd();
    const tmp = realpathSync(mkdtempSync(join(tmpdir(), 'vp-resolve-lint-')));
    try {
      writeFileSync(join(tmp, 'package.json'), '{"devDependencies":{"oxlint":"1.60.0"}}');
      mkdirSync(join(tmp, 'node_modules', 'oxlint'), { recursive: true });
      writeFileSync(join(tmp, 'node_modules', 'oxlint', 'package.json'), '{"name":"oxlint"}');
      writeFileSync(join(tmp, 'node_modules', 'oxlint', 'index.js'), '');
      mkdirSync(join(tmp, 'project'));
      writeFileSync(join(tmp, 'project', 'package.json'), '{}');
      process.chdir(join(tmp, 'project'));

      const { binPath } = await lint();

      expect(binPath.startsWith(tmp)).toBe(false);
    } finally {
      process.chdir(originalCwd);
      rmSync(tmp, { recursive: true, force: true });
    }
  });

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
