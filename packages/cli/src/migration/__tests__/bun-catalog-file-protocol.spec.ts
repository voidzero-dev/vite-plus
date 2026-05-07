import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { PackageManager, type WorkspaceInfo } from '../../types/index.js';

// Mock with file: protocol paths (simulating VP_OVERRIDE_PACKAGES in CI)
vi.mock('../../utils/constants.js', async (importOriginal) => {
  const mod = await importOriginal<typeof import('../../utils/constants.js')>();
  return {
    ...mod,
    VITE_PLUS_VERSION: 'file:/tmp/tgz/vite-plus-0.0.0.tgz',
    VITE_PLUS_OVERRIDE_PACKAGES: {
      vite: 'file:/tmp/tgz/voidzero-dev-vite-plus-core-0.0.0.tgz',
      vitest: 'file:/tmp/tgz/voidzero-dev-vite-plus-test-0.0.0.tgz',
      '@voidzero-dev/vite-plus-core': 'file:/tmp/tgz/voidzero-dev-vite-plus-core-0.0.0.tgz',
      '@voidzero-dev/vite-plus-test': 'file:/tmp/tgz/voidzero-dev-vite-plus-test-0.0.0.tgz',
    },
  };
});

const { rewriteMonorepo, rewritePackageJson } = await import('../migrator.js');

function makeWorkspaceInfo(rootDir: string, packageManager: PackageManager): WorkspaceInfo {
  return {
    rootDir,
    isMonorepo: false,
    monorepoScope: '',
    workspacePatterns: [],
    parentDirs: [],
    packageManager,
    packageManagerVersion: '10.33.0',
    downloadPackageManager: {
      name: packageManager,
      installDir: '/tmp',
      binPrefix: '/tmp/bin',
      packageName: packageManager,
      version: '1.0.0',
    },
    packages: [],
  };
}

function readJson(filePath: string): Record<string, unknown> {
  return JSON.parse(fs.readFileSync(filePath, 'utf8'));
}

describe('rewriteMonorepo bun catalog with file: protocol', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-test-bun-file-'));
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  it('uses file: paths directly in overrides instead of catalog:', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'bun-monorepo',
        workspaces: ['packages/*'],
        devDependencies: { vite: '^7.0.0' },
        packageManager: 'bun@1.3.11',
      }),
    );
    rewriteMonorepo(makeWorkspaceInfo(tmpDir, PackageManager.bun), true);

    const pkg = readJson(path.join(tmpDir, 'package.json'));
    // catalog should not contain file: entries
    const catalog = (pkg.catalog ?? {}) as Record<string, string>;
    expect(catalog.vite).toBeUndefined();
    expect(catalog.vitest).toBeUndefined();
    // overrides should use file: paths directly, not catalog:
    const overrides = pkg.overrides as Record<string, string>;
    expect(overrides.vite).toBe('file:/tmp/tgz/voidzero-dev-vite-plus-core-0.0.0.tgz');
    expect(overrides.vitest).toBe('file:/tmp/tgz/voidzero-dev-vite-plus-test-0.0.0.tgz');
    expect(overrides['@voidzero-dev/vite-plus-core']).toBe(
      'file:/tmp/tgz/voidzero-dev-vite-plus-core-0.0.0.tgz',
    );
    expect(overrides['@voidzero-dev/vite-plus-test']).toBe(
      'file:/tmp/tgz/voidzero-dev-vite-plus-test-0.0.0.tgz',
    );
  });

  it('does not write file: paths into named catalogs', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'bun-monorepo',
        workspaces: {
          packages: ['packages/*'],
          catalogs: {
            build: {
              vite: '^7.0.0',
              vitest: '^4.0.0',
              tsdown: '^0.1.0',
            },
          },
        },
        devDependencies: { vite: 'catalog:build' },
        overrides: { vite: 'catalog:build' },
        packageManager: 'bun@1.3.11',
      }),
    );

    rewriteMonorepo(makeWorkspaceInfo(tmpDir, PackageManager.bun), true);

    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      workspaces: {
        catalog: Record<string, string>;
        catalogs: Record<string, Record<string, string>>;
      };
      overrides: Record<string, string>;
      devDependencies: Record<string, string>;
    };
    expect(pkg.workspaces.catalog.vite).toBeUndefined();
    expect(pkg.workspaces.catalog.vitest).toBeUndefined();
    expect(pkg.workspaces.catalogs.build.vite).toBe('^7.0.0');
    expect(pkg.workspaces.catalogs.build.vitest).toBe('^4.0.0');
    expect(pkg.workspaces.catalogs.build.tsdown).toBeUndefined();
    expect(pkg.overrides.vite).toBe('file:/tmp/tgz/voidzero-dev-vite-plus-core-0.0.0.tgz');
    expect(pkg.devDependencies.vite).toBe('file:/tmp/tgz/voidzero-dev-vite-plus-core-0.0.0.tgz');
  });

  it('does not write file: paths into peer dependencies', () => {
    const pkg = {
      peerDependencies: {
        vite: '^7.0.0',
        vitest: 'catalog:test',
      },
      optionalDependencies: {
        vite: '^7.0.0',
      },
    };

    rewritePackageJson(pkg, PackageManager.pnpm, true);

    expect(pkg.peerDependencies.vite).toBe('^7.0.0');
    expect(pkg.peerDependencies.vitest).toBe('*');
    expect(pkg.optionalDependencies.vite).toBe(
      'file:/tmp/tgz/voidzero-dev-vite-plus-core-0.0.0.tgz',
    );
    expect(
      (pkg as { devDependencies?: Record<string, string> }).devDependencies?.['vite-plus'],
    ).toBe('file:/tmp/tgz/vite-plus-0.0.0.tgz');
  });
});
