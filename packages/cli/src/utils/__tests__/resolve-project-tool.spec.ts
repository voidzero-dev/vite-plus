import { mkdirSync, mkdtempSync, realpathSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path, { join } from 'node:path';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { resolveBundled } from '../constants.js';
import { resolveProjectTool } from '../resolve-project-tool.js';
import { dirsUpTo, findPackageRoot } from '../workspace-root.js';

function writeFakeOxlint(nodeModulesDir: string) {
  const pkgDir = join(nodeModulesDir, 'oxlint');
  mkdirSync(join(pkgDir, 'dist'), { recursive: true });
  writeFileSync(
    join(pkgDir, 'package.json'),
    JSON.stringify({ name: 'oxlint', version: '0.0.0-fake', main: 'dist/index.js' }),
  );
  writeFileSync(join(pkgDir, 'dist', 'index.js'), '');
  return join(pkgDir, 'dist', 'index.js');
}

function writePackageJson(dir: string, pkg: Record<string, unknown>) {
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, 'package.json'), JSON.stringify(pkg));
}

const declaresOxlint = { devDependencies: { oxlint: '1.60.0' } };

describe('resolveProjectTool', () => {
  let tmp: string;
  let bundled: string;

  beforeEach(() => {
    tmp = realpathSync(mkdtempSync(join(tmpdir(), 'vp-project-tool-')));
    bundled = resolveBundled('oxlint');
  });

  afterEach(() => {
    rmSync(tmp, { recursive: true, force: true });
    delete process.versions.pnp;
  });

  it('uses the copy declared and installed at the pnpm workspace root from a workspace package', () => {
    writePackageJson(tmp, { name: 'root', ...declaresOxlint });
    writeFileSync(join(tmp, 'pnpm-workspace.yaml'), 'packages:\n  - packages/*\n');
    const rootOxlint = writeFakeOxlint(join(tmp, 'node_modules'));
    const pkg = join(tmp, 'packages', 'a');
    writePackageJson(pkg, { name: 'a' });

    expect(resolveProjectTool('oxlint', pkg)).toBe(rootOxlint);
  });

  it('finds a copy that npm hoisted to the workspace root for a package that declares it', () => {
    writePackageJson(tmp, { name: 'root', workspaces: ['packages/*'] });
    const rootOxlint = writeFakeOxlint(join(tmp, 'node_modules'));
    const pkg = join(tmp, 'packages', 'a');
    writePackageJson(pkg, { name: 'a', ...declaresOxlint });

    expect(resolveProjectTool('oxlint', pkg)).toBe(rootOxlint);
  });

  it('ignores a copy installed in the project that the project does not declare', () => {
    writePackageJson(tmp, { name: 'app' });
    writeFakeOxlint(join(tmp, 'node_modules'));

    expect(resolveProjectTool('oxlint', tmp)).toBe(bundled);
  });

  it('ignores a copy declared by a package.json above the nearest package when no workspace links them', () => {
    writePackageJson(tmp, { name: 'outer', ...declaresOxlint });
    writeFakeOxlint(join(tmp, 'node_modules'));
    const pkg = join(tmp, 'packages', 'a');
    writePackageJson(pkg, { name: 'a' });

    expect(resolveProjectTool('oxlint', pkg)).toBe(bundled);
  });

  it('ignores a declared copy that is only installed above the workspace root', () => {
    writePackageJson(tmp, { name: 'outer', ...declaresOxlint });
    writeFakeOxlint(join(tmp, 'node_modules'));
    const project = join(tmp, 'project');
    writePackageJson(project, { name: 'app', ...declaresOxlint });

    expect(resolveProjectTool('oxlint', project)).toBe(bundled);
  });

  it('treats a package under a pnpm workspace root as part of that workspace, like vp run', () => {
    writePackageJson(tmp, { name: 'root', ...declaresOxlint });
    writeFileSync(join(tmp, 'pnpm-workspace.yaml'), 'packages:\n  - packages/*\n');
    const rootOxlint = writeFakeOxlint(join(tmp, 'node_modules'));
    const standalone = join(tmp, 'tools', 'standalone');
    writePackageJson(standalone, { name: 'standalone' });

    expect(resolveProjectTool('oxlint', standalone)).toBe(rootOxlint);
  });

  it('uses the bundled copy outside any package', () => {
    const loose = join(tmp, 'loose');
    mkdirSync(loose);
    writeFakeOxlint(join(tmp, 'node_modules'));

    expect(findPackageRoot(loose)).toBeUndefined();
    expect(resolveProjectTool('oxlint', loose)).toBe(bundled);
  });

  it('keeps the declaration check under Yarn PnP', () => {
    process.versions.pnp = '3';
    const project = join(tmp, 'project');
    writePackageJson(project, { name: 'app', ...declaresOxlint });
    const projectOxlint = writeFakeOxlint(join(project, 'node_modules'));
    const other = join(tmp, 'other');
    writePackageJson(other, { name: 'other' });

    expect(resolveProjectTool('oxlint', project)).toBe(projectOxlint);
    expect(resolveProjectTool('oxlint', other)).toBe(bundled);
  });
});

describe('dirsUpTo', () => {
  it('stops at the given ancestor', () => {
    expect([...dirsUpTo('C:\\repo\\packages\\a', 'C:\\repo', path.win32)]).toEqual([
      'C:\\repo\\packages\\a',
      'C:\\repo\\packages',
      'C:\\repo',
    ]);
  });

  it('stops at a Windows drive root', () => {
    expect([...dirsUpTo('C:\\repo\\a', undefined, path.win32)]).toEqual([
      'C:\\repo\\a',
      'C:\\repo',
      'C:\\',
    ]);
  });

  it('stops at a Windows UNC share root', () => {
    expect([...dirsUpTo('\\\\server\\share\\repo', undefined, path.win32)]).toEqual([
      '\\\\server\\share\\repo',
      '\\\\server\\share\\',
    ]);
  });

  it('stops at the root when the ancestor is not on the path', () => {
    expect([...dirsUpTo('C:\\repo\\a', 'D:\\other', path.win32)]).toEqual([
      'C:\\repo\\a',
      'C:\\repo',
      'C:\\',
    ]);
    expect([...dirsUpTo('/repo/a', '/other', path.posix)]).toEqual(['/repo/a', '/repo', '/']);
  });
});
