import { existsSync, mkdirSync, mkdtempSync, realpathSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { lint, resolveWindowsLintBackendExecutable } from '../resolve-lint.ts';

function writeFakeOxlint(nodeModulesDir: string) {
  const pkgDir = join(nodeModulesDir, 'oxlint');
  mkdirSync(join(pkgDir, 'dist'), { recursive: true });
  writeFileSync(
    join(pkgDir, 'package.json'),
    JSON.stringify({ name: 'oxlint', version: '0.0.0-fake', main: 'dist/index.js' }),
  );
  writeFileSync(join(pkgDir, 'dist', 'index.js'), '');
  return pkgDir;
}

function writePackageJson(dir: string, pkg: Record<string, unknown>) {
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, 'package.json'), JSON.stringify(pkg));
}

describe('resolve-lint', () => {
  describe('which oxlint runs', () => {
    let originalCwd: string;
    let tmp: string;

    beforeEach(() => {
      originalCwd = process.cwd();
      tmp = realpathSync(mkdtempSync(join(tmpdir(), 'vp-resolve-lint-')));
    });

    afterEach(() => {
      process.chdir(originalCwd);
      rmSync(tmp, { recursive: true, force: true });
    });

    it('runs the bundled oxlint, not one installed in a parent directory such as the main checkout around a git worktree', async () => {
      writePackageJson(tmp, { name: 'parent', devDependencies: { oxlint: '1.60.0' } });
      writeFakeOxlint(join(tmp, 'node_modules'));
      const project = join(tmp, 'project');
      writePackageJson(project, { name: 'app', devDependencies: { 'vite-plus': '1.2.0' } });
      process.chdir(project);

      const { binPath } = await lint();

      expect(binPath.startsWith(tmp)).toBe(false);
      expect(existsSync(binPath)).toBe(true);
    });

    it('runs the oxlint the project declares', async () => {
      const project = join(tmp, 'project');
      writePackageJson(project, { name: 'app', devDependencies: { oxlint: '1.60.0' } });
      const fakeOxlint = writeFakeOxlint(join(project, 'node_modules'));
      process.chdir(project);

      const { binPath } = await lint();

      expect(binPath).toBe(join(fakeOxlint, 'bin', 'oxlint'));
    });
  });

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
    const backendPath = String.raw`C:\repo\node_modules\.pnpm\vite-plus@0.1.24\node_modules\vite-plus\node_modules\.bin\tsrs-lint.cmd`;
    const result = resolveWindowsLintBackendExecutable(
      [
        String.raw`C:\repo\node_modules\.pnpm\vite-plus@0.1.24\node_modules\vite-plus\node_modules\.bin\tsrs-lint.exe`,
        backendPath,
      ],
      {
        exists: (path) => path === backendPath,
      },
    );

    expect(result).toBe(backendPath);
    expect(result).not.toMatch(/^\.\\/);
  });

  it('prefers the executable wrapper without resolving Bun paths', () => {
    const candidates = [
      String.raw`C:\repo\.bin\tsrs-lint.exe`,
      String.raw`C:\repo\.bin\tsrs-lint.cmd`,
    ];
    const getRealpathCandidates = vi.fn<() => Array<string>>(() => [
      String.raw`C:\cache\.bin\tsrs-lint.exe`,
    ]);

    expect(
      resolveWindowsLintBackendExecutable(candidates, {
        exists: () => true,
        getRealpathCandidates,
      }),
    ).toBe(candidates[0]);
    expect(getRealpathCandidates).not.toHaveBeenCalled();
  });

  it('resolves a Bun wrapper when the symlinked paths do not exist', () => {
    const backendPath = String.raw`C:\repo\node_modules\.bun\tsrs\node_modules\.bin\tsrs-lint.exe`;

    expect(
      resolveWindowsLintBackendExecutable([String.raw`C:\repo\.bin\tsrs-lint.exe`], {
        exists: (path) => path === backendPath,
        getRealpathCandidates: () => [backendPath],
      }),
    ).toBe(backendPath);
  });

  it.each([
    { name: 'absent', getRealpathCandidates: undefined },
    { name: 'empty', getRealpathCandidates: () => [] },
    {
      name: 'unavailable',
      getRealpathCandidates: () => {
        throw new Error('realpath failed');
      },
    },
  ])('reports the original candidates when the fallback is $name', ({ getRealpathCandidates }) => {
    const candidates = [
      String.raw`C:\repo\.bin\tsrs-lint.exe`,
      String.raw`C:\repo\.bin\tsrs-lint.cmd`,
    ];

    expect(() =>
      resolveWindowsLintBackendExecutable(candidates, {
        exists: () => false,
        getRealpathCandidates,
      }),
    ).toThrow(
      `Unable to resolve type-aware lint executable, tried:\n- ${candidates[0]}\n- ${candidates[1]}`,
    );
  });
});
