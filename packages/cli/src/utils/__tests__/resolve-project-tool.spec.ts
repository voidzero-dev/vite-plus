import { mkdirSync, mkdtempSync, realpathSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { resolveBundled } from '../constants.js';
import { resolveProjectTool } from '../resolve-project-tool.js';

function writePackageJson(dir: string, pkg: Record<string, unknown>) {
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, 'package.json'), JSON.stringify(pkg));
}

function installOxlint(dir: string) {
  const pkgDir = join(dir, 'node_modules', 'oxlint');
  writePackageJson(pkgDir, { name: 'oxlint' });
  writeFileSync(join(pkgDir, 'index.js'), '');
  return join(pkgDir, 'index.js');
}

describe('resolveProjectTool', () => {
  let tmp: string;

  beforeEach(() => {
    tmp = realpathSync(mkdtempSync(join(tmpdir(), 'vp-project-tool-')));
  });

  afterEach(() => {
    rmSync(tmp, { recursive: true, force: true });
  });

  it('should ignore an oxlint installed in a parent directory', () => {
    writePackageJson(tmp, { devDependencies: { oxlint: '1.60.0' } });
    installOxlint(tmp);
    writePackageJson(join(tmp, 'project'), {});

    expect(resolveProjectTool('oxlint', join(tmp, 'project'))).toBe(resolveBundled('oxlint'));
  });

  it('should ignore an oxlint the project installs but does not declare', () => {
    writePackageJson(tmp, {});
    installOxlint(tmp);

    expect(resolveProjectTool('oxlint', tmp)).toBe(resolveBundled('oxlint'));
  });

  it('should use the oxlint the project declares', () => {
    writePackageJson(tmp, { devDependencies: { oxlint: '1.60.0' } });
    const oxlint = installOxlint(tmp);

    expect(resolveProjectTool('oxlint', tmp)).toBe(oxlint);
  });

  it('should use the oxlint the workspace root declares from a workspace package', () => {
    writePackageJson(tmp, { devDependencies: { oxlint: '1.60.0' } });
    writeFileSync(join(tmp, 'pnpm-workspace.yaml'), 'packages:\n  - packages/*\n');
    writePackageJson(join(tmp, 'packages', 'a'), {});
    const oxlint = installOxlint(tmp);

    expect(resolveProjectTool('oxlint', join(tmp, 'packages', 'a'))).toBe(oxlint);
  });
});
