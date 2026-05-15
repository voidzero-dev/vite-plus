import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { parse as parseYaml } from 'yaml';

import { DependencyType, PackageManager, type WorkspaceInfo } from '../../types/index.ts';
import {
  discoverWorkspacePackages,
  findPackageJsonFilesFromPatterns,
  updatePackageJsonWithDeps,
  updateWorkspaceConfig,
} from '../workspace.ts';

let tmpDir: string;

beforeEach(() => {
  tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-workspace-test-'));
});

afterEach(() => {
  fs.rmSync(tmpDir, { recursive: true, force: true });
});

function writeJson(p: string, data: unknown) {
  fs.mkdirSync(path.dirname(p), { recursive: true });
  fs.writeFileSync(p, JSON.stringify(data, null, 2));
}

function makeWorkspaceInfo(overrides: Partial<WorkspaceInfo> = {}): WorkspaceInfo {
  return {
    rootDir: tmpDir,
    packageManager: PackageManager.pnpm,
    packageManagerVersion: 'latest',
    isMonorepo: true,
    monorepoScope: '',
    workspacePatterns: [],
    parentDirs: [],
    packages: [],
    downloadPackageManager: {
      name: '',
      installDir: '',
      binPrefix: '',
      packageName: '',
      version: '',
    },
    ...overrides,
  };
}

describe('updateWorkspaceConfig pattern derivation', () => {
  it('derives "packages/*" from a nested project path (pnpm)', () => {
    fs.writeFileSync(path.join(tmpDir, 'pnpm-workspace.yaml'), "packages:\n  - 'apps/*'\n");
    updateWorkspaceConfig('packages/my-app', makeWorkspaceInfo({ workspacePatterns: ['apps/*'] }));
    const content = fs.readFileSync(path.join(tmpDir, 'pnpm-workspace.yaml'), 'utf8');
    const parsed = parseYaml(content) as { packages: string[] };
    expect(parsed.packages).toEqual(['apps/*', 'packages/*']);
  });

  it('derives "foo/bar/*" from a deeply nested project path (pnpm)', () => {
    fs.writeFileSync(path.join(tmpDir, 'pnpm-workspace.yaml'), 'packages: []\n');
    updateWorkspaceConfig('foo/bar/app', makeWorkspaceInfo({ workspacePatterns: [] }));
    const parsed = parseYaml(fs.readFileSync(path.join(tmpDir, 'pnpm-workspace.yaml'), 'utf8')) as {
      packages: string[];
    };
    expect(parsed.packages).toEqual(['foo/bar/*']);
  });

  it('keeps a single-segment project path literal (pnpm)', () => {
    fs.writeFileSync(path.join(tmpDir, 'pnpm-workspace.yaml'), 'packages: []\n');
    updateWorkspaceConfig('website', makeWorkspaceInfo({ workspacePatterns: [] }));
    const parsed = parseYaml(fs.readFileSync(path.join(tmpDir, 'pnpm-workspace.yaml'), 'utf8')) as {
      packages: string[];
    };
    expect(parsed.packages).toEqual(['website']);
  });

  it('skips update when the project path already matches an existing pattern', () => {
    fs.writeFileSync(path.join(tmpDir, 'pnpm-workspace.yaml'), "packages:\n  - 'apps/*'\n");
    updateWorkspaceConfig('apps/admin', makeWorkspaceInfo({ workspacePatterns: ['apps/*'] }));
    const parsed = parseYaml(fs.readFileSync(path.join(tmpDir, 'pnpm-workspace.yaml'), 'utf8')) as {
      packages: string[];
    };
    expect(parsed.packages).toEqual(['apps/*']);
  });

  it.each([PackageManager.npm, PackageManager.bun, PackageManager.yarn])(
    'skips update when the project path already matches an existing pattern (%s)',
    (packageManager) => {
      writeJson(path.join(tmpDir, 'package.json'), {
        name: 'root',
        workspaces: ['apps/*'],
      });
      updateWorkspaceConfig(
        'apps/admin',
        makeWorkspaceInfo({ packageManager, workspacePatterns: ['apps/*'] }),
      );
      const pkg = JSON.parse(fs.readFileSync(path.join(tmpDir, 'package.json'), 'utf8'));
      expect(pkg.workspaces).toEqual(['apps/*']);
    },
  );

  it('initializes packages key when pnpm-workspace.yaml has no packages field', () => {
    fs.writeFileSync(path.join(tmpDir, 'pnpm-workspace.yaml'), 'catalog:\n  foo: ^1.0.0\n');
    updateWorkspaceConfig('website', makeWorkspaceInfo({ workspacePatterns: [] }));
    const parsed = parseYaml(fs.readFileSync(path.join(tmpDir, 'pnpm-workspace.yaml'), 'utf8')) as {
      packages: string[];
      catalog: Record<string, string>;
    };
    expect(parsed.packages).toEqual(['website']);
    expect(parsed.catalog).toEqual({ foo: '^1.0.0' });
  });

  it('initializes workspaces field when package.json has none (npm)', () => {
    writeJson(path.join(tmpDir, 'package.json'), { name: 'root' });
    updateWorkspaceConfig(
      'packages/foo',
      makeWorkspaceInfo({ packageManager: PackageManager.npm, workspacePatterns: [] }),
    );
    const pkg = JSON.parse(fs.readFileSync(path.join(tmpDir, 'package.json'), 'utf8'));
    expect(pkg.workspaces).toEqual(['packages/*']);
  });

  it('appends to npm-style array workspaces in package.json', () => {
    writeJson(path.join(tmpDir, 'package.json'), {
      name: 'root',
      workspaces: ['apps/*'],
    });
    updateWorkspaceConfig(
      'packages/foo',
      makeWorkspaceInfo({
        packageManager: PackageManager.npm,
        workspacePatterns: ['apps/*'],
      }),
    );
    const pkg = JSON.parse(fs.readFileSync(path.join(tmpDir, 'package.json'), 'utf8'));
    expect(pkg.workspaces).toEqual(['apps/*', 'packages/*']);
  });

  it('preserves object-form workspaces (bun catalogs / yarn nohoist)', () => {
    writeJson(path.join(tmpDir, 'package.json'), {
      name: 'root',
      workspaces: {
        packages: ['apps/*'],
        catalog: { react: '^19.0.0' },
      },
    });
    updateWorkspaceConfig(
      'packages/foo',
      makeWorkspaceInfo({
        packageManager: PackageManager.bun,
        workspacePatterns: ['apps/*'],
      }),
    );
    const pkg = JSON.parse(fs.readFileSync(path.join(tmpDir, 'package.json'), 'utf8'));
    expect(pkg.workspaces).toEqual({
      packages: ['apps/*', 'packages/*'],
      catalog: { react: '^19.0.0' },
    });
  });
});

describe('discoverWorkspacePackages', () => {
  it('returns an empty list when no patterns are provided', () => {
    expect(discoverWorkspacePackages([], tmpDir)).toEqual([]);
  });

  it('finds packages matching workspace patterns', () => {
    writeJson(path.join(tmpDir, 'packages/foo/package.json'), {
      name: 'foo',
      version: '1.0.0',
      description: 'a foo',
    });
    writeJson(path.join(tmpDir, 'packages/bar/package.json'), {
      name: 'bar',
      version: '2.0.0',
    });
    // package.json without "name" must be skipped
    writeJson(path.join(tmpDir, 'packages/nameless/package.json'), {
      version: '0.0.0',
    });

    const packages = discoverWorkspacePackages(['packages/*'], tmpDir);
    const names = packages.map((p) => p.name).toSorted();
    expect(names).toEqual(['bar', 'foo']);
    const foo = packages.find((p) => p.name === 'foo')!;
    expect(foo.path).toBe(path.join('packages', 'foo'));
    expect(foo.description).toBe('a foo');
    expect(foo.version).toBe('1.0.0');
    expect(foo.isTemplatePackage).toBe(false);
  });

  it('flags packages keyworded as vite-plus-template / bingo-template', () => {
    writeJson(path.join(tmpDir, 'pkgs/vp/package.json'), {
      name: 'vp',
      keywords: ['vite-plus-template'],
    });
    writeJson(path.join(tmpDir, 'pkgs/bg/package.json'), {
      name: 'bg',
      keywords: ['bingo-template'],
    });
    writeJson(path.join(tmpDir, 'pkgs/bd/package.json'), {
      name: 'bd',
      dependencies: { bingo: '*' },
    });
    writeJson(path.join(tmpDir, 'pkgs/plain/package.json'), { name: 'plain' });

    const packages = discoverWorkspacePackages(['pkgs/*'], tmpDir);
    const map = Object.fromEntries(packages.map((p) => [p.name, p.isTemplatePackage]));
    expect(map).toEqual({ vp: true, bg: true, bd: true, plain: false });
  });

  it('ignores node_modules during discovery', () => {
    writeJson(path.join(tmpDir, 'packages/a/package.json'), { name: 'a' });
    writeJson(path.join(tmpDir, 'packages/a/node_modules/dep/package.json'), {
      name: 'dep',
    });
    const packages = discoverWorkspacePackages(['packages/*'], tmpDir);
    expect(packages.map((p) => p.name)).toEqual(['a']);
  });
});

describe('updatePackageJsonWithDeps', () => {
  it('adds workspace:* deps under the requested dependency type', () => {
    writeJson(path.join(tmpDir, 'apps/app/package.json'), { name: 'app' });
    updatePackageJsonWithDeps(tmpDir, 'apps/app', ['shared', 'ui'], DependencyType.dependencies);
    const pkg = JSON.parse(fs.readFileSync(path.join(tmpDir, 'apps/app/package.json'), 'utf8'));
    expect(pkg.dependencies).toEqual({ shared: 'workspace:*', ui: 'workspace:*' });
  });

  it('preserves existing entries of the same dependency type', () => {
    writeJson(path.join(tmpDir, 'apps/app/package.json'), {
      name: 'app',
      devDependencies: { existing: '^1.0.0' },
    });
    updatePackageJsonWithDeps(tmpDir, 'apps/app', ['lint'], DependencyType.devDependencies);
    const pkg = JSON.parse(fs.readFileSync(path.join(tmpDir, 'apps/app/package.json'), 'utf8'));
    expect(pkg.devDependencies).toEqual({ existing: '^1.0.0', lint: 'workspace:*' });
  });
});

describe('findPackageJsonFilesFromPatterns', () => {
  it('returns absolute paths for matching package.json files', () => {
    writeJson(path.join(tmpDir, 'packages/a/package.json'), { name: 'a' });
    writeJson(path.join(tmpDir, 'packages/b/package.json'), { name: 'b' });
    const result = findPackageJsonFilesFromPatterns(['packages/*'], tmpDir).toSorted();
    expect(result).toEqual(
      [
        path.join(tmpDir, 'packages/a/package.json'),
        path.join(tmpDir, 'packages/b/package.json'),
      ].toSorted(),
    );
  });

  it('returns empty when given no patterns', () => {
    expect(findPackageJsonFilesFromPatterns([], tmpDir)).toEqual([]);
  });
});
