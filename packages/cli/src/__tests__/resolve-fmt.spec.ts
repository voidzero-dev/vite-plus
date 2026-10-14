import { existsSync, mkdirSync, mkdtempSync, realpathSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { fmt } from '../resolve-fmt.js';

function writeFakeOxfmt(nodeModulesDir: string) {
  const pkgDir = join(nodeModulesDir, 'oxfmt');
  mkdirSync(join(pkgDir, 'dist'), { recursive: true });
  writeFileSync(
    join(pkgDir, 'package.json'),
    JSON.stringify({ name: 'oxfmt', version: '0.0.0-fake', main: 'dist/index.js' }),
  );
  writeFileSync(join(pkgDir, 'dist', 'index.js'), '');
  return pkgDir;
}

function writePackageJson(dir: string, pkg: Record<string, unknown>) {
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, 'package.json'), JSON.stringify(pkg));
}

describe('resolve-fmt', () => {
  let originalCwd: string;
  let tmp: string;

  beforeEach(() => {
    originalCwd = process.cwd();
    tmp = realpathSync(mkdtempSync(join(tmpdir(), 'vp-resolve-fmt-')));
  });

  afterEach(() => {
    process.chdir(originalCwd);
    rmSync(tmp, { recursive: true, force: true });
  });

  it('runs the bundled oxfmt, not one installed in a parent directory', async () => {
    writePackageJson(tmp, { name: 'parent', devDependencies: { oxfmt: '0.45.0' } });
    writeFakeOxfmt(join(tmp, 'node_modules'));
    const project = join(tmp, 'project');
    writePackageJson(project, { name: 'app', devDependencies: { 'vite-plus': '1.2.0' } });
    process.chdir(project);

    const { binPath } = await fmt();

    expect(binPath.startsWith(tmp)).toBe(false);
    expect(existsSync(binPath)).toBe(true);
  });

  it('runs the oxfmt the project declares', async () => {
    const project = join(tmp, 'project');
    writePackageJson(project, { name: 'app', devDependencies: { oxfmt: '0.45.0' } });
    const fakeOxfmt = writeFakeOxfmt(join(project, 'node_modules'));
    process.chdir(project);

    const { binPath } = await fmt();

    expect(binPath).toBe(join(fakeOxfmt, 'bin', 'oxfmt'));
  });
});
