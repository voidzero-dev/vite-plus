import { existsSync, mkdirSync, mkdtempSync, realpathSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { parentTestConfigDiagnostic, test } from '../resolve-test.js';
import { resolve, resolveBundled } from '../utils/constants.js';

describe('resolve-test', () => {
  describe('parent config diagnostic', () => {
    let dir: string;
    beforeEach(() => {
      dir = mkdtempSync(join(tmpdir(), 'vp-parent-test-config-'));
      mkdirSync(join(dir, 'sub'));
      writeFileSync(join(dir, 'vite.config.ts'), 'export default {}');
    });
    afterEach(() => rmSync(dir, { recursive: true, force: true }));

    it('reports a parent config without selecting it', () => {
      expect(parentTestConfigDiagnostic(join(dir, 'sub'), ['run'])).toBe(
        'No test config was found in this directory.\n' +
          'A config exists at ../vite.config.ts. Run `vp test --config ../vite.config.ts --dir .`.',
      );
    });

    it.each([
      ['--config', '../vite.config.ts'],
      ['--config=../vite.config.ts'],
      ['-c', '../vite.config.ts'],
      ['--help'],
      ['--root', '..'],
    ])('respects explicit options %j', (...args) => {
      expect(parentTestConfigDiagnostic(join(dir, 'sub'), args)).toBeNull();
    });

    it('prefers a config in the selected directory', () => {
      writeFileSync(join(dir, 'sub', 'vitest.config.mts'), 'export default {}');
      expect(parentTestConfigDiagnostic(join(dir, 'sub'), [])).toBeNull();
    });
  });

  it('resolves a vitest binary that exists', async () => {
    const { binPath } = await test();

    expect(binPath).toBeTruthy();
    expect(
      existsSync(binPath),
      `vitest binPath should point to an existing file, got: ${binPath}`,
    ).toBe(true);
    expect(binPath).toContain('vitest');
  });

  // `vp test` MUST run the Vitest bundled with the CLI, not a project-local
  // copy: the `vite-plus/test*` shims `export * from 'vitest'`, which Node
  // resolves to vite-plus's own bundled Vitest. Spawning a different physical
  // Vitest as the runner yields two copies in one run — a classic source of
  // internal-state / mock-hoisting mismatches.
  describe('prefers the bundled vitest over a project-local copy', () => {
    let originalCwd: string;
    let projectDir: string;

    beforeEach(() => {
      originalCwd = process.cwd();
      // realpath so comparisons match `require.resolve` output, which resolves
      // symlinks (macOS `tmpdir()` is /var → /private/var).
      projectDir = realpathSync(mkdtempSync(join(tmpdir(), 'vp-resolve-test-')));
      // A fake project-local vitest that must NOT be selected as the runner.
      const fakeVitest = join(projectDir, 'node_modules', 'vitest');
      mkdirSync(fakeVitest, { recursive: true });
      writeFileSync(
        join(fakeVitest, 'package.json'),
        JSON.stringify({
          name: 'vitest',
          version: '0.0.0-fake-project-copy',
          bin: { vitest: 'fake.mjs' },
        }),
      );
      writeFileSync(join(fakeVitest, 'fake.mjs'), '');
      process.chdir(projectDir);
    });

    afterEach(() => {
      process.chdir(originalCwd);
      rmSync(projectDir, { recursive: true, force: true });
    });

    it('runs the bundled vitest, never the project copy', async () => {
      const { binPath } = await test();

      expect(
        binPath.startsWith(projectDir),
        `runner should be the bundled vitest, not the project copy at ${projectDir}, got: ${binPath}`,
      ).toBe(false);
      expect(binPath).not.toContain('0.0.0-fake');
      expect(existsSync(binPath)).toBe(true);
    });

    it('contrasts with CWD-first resolve(), which would pick the project copy', () => {
      // The fixture genuinely shadows: the old CWD-first helper finds the fake…
      expect(resolve('vitest/package.json').startsWith(projectDir)).toBe(true);
      // …while the bundle-first helper does not.
      expect(resolveBundled('vitest/package.json').startsWith(projectDir)).toBe(false);
    });
  });
});
