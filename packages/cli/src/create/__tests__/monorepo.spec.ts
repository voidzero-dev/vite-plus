import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { PackageManager } from '../../types/index.js';
import {
  alignMonorepoTypeScriptVersion,
  dropAliasedRuntimeDevDeps,
} from '../templates/monorepo.js';

function writePackageJson(directory: string, devDependencies: Record<string, string>): void {
  fs.writeFileSync(
    path.join(directory, 'package.json'),
    JSON.stringify({ private: true, devDependencies }, null, 2),
  );
}

function readDevDependencies(directory: string): Record<string, string> {
  const pkg = JSON.parse(fs.readFileSync(path.join(directory, 'package.json'), 'utf8')) as {
    devDependencies?: Record<string, string>;
  };
  return pkg.devDependencies ?? {};
}

describe('alignMonorepoTypeScriptVersion', () => {
  let tmpDir: string;
  let appDir: string;
  let libraryDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-monorepo-typescript-'));
    appDir = path.join(tmpDir, 'apps', 'website');
    libraryDir = path.join(tmpDir, 'packages', 'utils');
    fs.mkdirSync(appDir, { recursive: true });
    fs.mkdirSync(libraryDir, { recursive: true });
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  // Regression test for the pnpm single-instance invariant: the app and
  // library templates are updated independently upstream, and when their
  // typescript devDeps resolve to different versions, pnpm materializes two
  // vite-plus/vitest peer instances in the generated workspace.
  it('aligns the app with the library TypeScript range', () => {
    writePackageJson(appDir, { typescript: '~6.0.2', 'vite-plus': 'catalog:' });
    writePackageJson(libraryDir, {
      typescript: '^7.0.2',
      '@types/node': '^26.1.1',
      'vite-plus': 'catalog:',
    });

    alignMonorepoTypeScriptVersion(tmpDir, appDir, libraryDir);

    expect(readDevDependencies(appDir)).toEqual({
      typescript: '^7.0.2',
      'vite-plus': 'catalog:',
    });
  });

  it('leaves the app unchanged when it has no TypeScript dependency', () => {
    writePackageJson(appDir, { 'vite-plus': 'catalog:' });
    writePackageJson(libraryDir, { typescript: '^7.0.2', 'vite-plus': 'catalog:' });

    alignMonorepoTypeScriptVersion(tmpDir, appDir, libraryDir);

    expect(readDevDependencies(appDir).typescript).toBeUndefined();
  });

  it('does nothing when the library has no TypeScript dependency', () => {
    writePackageJson(appDir, { typescript: '~6.0.2', 'vite-plus': 'catalog:' });
    writePackageJson(libraryDir, { 'vite-plus': 'catalog:' });

    alignMonorepoTypeScriptVersion(tmpDir, appDir, libraryDir);

    expect(readDevDependencies(appDir).typescript).toBe('~6.0.2');
  });

  it('updates the typescript entry of workspace catalogs to the library range', () => {
    writePackageJson(appDir, { typescript: '~6.0.2', 'vite-plus': 'catalog:' });
    writePackageJson(libraryDir, { typescript: '^7.0.2', 'vite-plus': 'catalog:' });
    fs.writeFileSync(
      path.join(tmpDir, 'pnpm-workspace.yaml'),
      'packages:\n  - apps/*\n\ncatalogMode: prefer\n\ncatalog:\n  typescript: ^5\n',
    );
    fs.writeFileSync(
      path.join(tmpDir, '.yarnrc.yml'),
      'nodeLinker: node-modules\ncatalog:\n  typescript: ^5\n',
    );

    alignMonorepoTypeScriptVersion(tmpDir, appDir, libraryDir);

    expect(fs.readFileSync(path.join(tmpDir, 'pnpm-workspace.yaml'), 'utf8')).toContain(
      'typescript: ^7.0.2',
    );
    expect(fs.readFileSync(path.join(tmpDir, '.yarnrc.yml'), 'utf8')).toContain(
      'typescript: ^7.0.2',
    );
  });

  it('leaves catalogs without a typescript entry untouched', () => {
    writePackageJson(appDir, { typescript: '~6.0.2', 'vite-plus': 'catalog:' });
    writePackageJson(libraryDir, { typescript: '^7.0.2', 'vite-plus': 'catalog:' });
    const workspaceYaml = 'packages:\n  - apps/*\n\ncatalog:\n  vite-plus: 0.2.4\n';
    fs.writeFileSync(path.join(tmpDir, 'pnpm-workspace.yaml'), workspaceYaml);

    alignMonorepoTypeScriptVersion(tmpDir, appDir, libraryDir);

    expect(fs.readFileSync(path.join(tmpDir, 'pnpm-workspace.yaml'), 'utf8')).toBe(workspaceYaml);
  });
});

describe('dropAliasedRuntimeDevDeps', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-monorepo-strip-'));
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  function writeWebsitePackageJson(devDependencies: Record<string, string>): void {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({ name: 'website', private: true, devDependencies }, null, 2),
    );
  }

  function readDevDependencies(): Record<string, string> {
    const pkg = JSON.parse(fs.readFileSync(path.join(tmpDir, 'package.json'), 'utf8')) as {
      devDependencies?: Record<string, string>;
    };
    return pkg.devDependencies ?? {};
  }

  // Regression test for "vp why vite reports the override as ineffective" in a
  // freshly created pnpm monorepo: pnpm only surfaces the pnpm-workspace.yaml
  // `overrides` through a package that directly depends on `vite`/`vitest`, so
  // the aliased (catalog:) devDeps must survive for the override to be
  // observable. Dropping them leaves `vite` resolving to upstream vite instead
  // of @voidzero-dev/vite-plus-core.
  it('keeps aliased vite/vitest for pnpm so the workspace override stays effective', () => {
    writeWebsitePackageJson({
      vite: 'catalog:',
      vitest: 'catalog:',
      'vite-plus': 'catalog:',
      typescript: '~6.0.2',
    });

    dropAliasedRuntimeDevDeps(tmpDir, PackageManager.pnpm);

    const devDependencies = readDevDependencies();
    expect(devDependencies.vite).toBe('catalog:');
    expect(devDependencies.vitest).toBe('catalog:');
    expect(devDependencies['vite-plus']).toBe('catalog:');
  });

  // npm/yarn/bun redirect the transitive/peer `vite` to
  // @voidzero-dev/vite-plus-core via root overrides/resolutions regardless of a
  // direct dependency, so the aliased keys are dead weight and stay dropped.
  for (const packageManager of [PackageManager.npm, PackageManager.yarn, PackageManager.bun]) {
    it(`drops aliased vite/vitest for ${packageManager}`, () => {
      writeWebsitePackageJson({
        vite: 'npm:@voidzero-dev/vite-plus-core@latest',
        vitest: 'npm:@voidzero-dev/vite-plus-test@latest',
        'vite-plus': 'latest',
        typescript: '~6.0.2',
      });

      dropAliasedRuntimeDevDeps(tmpDir, packageManager);

      const devDependencies = readDevDependencies();
      expect(devDependencies.vite).toBeUndefined();
      expect(devDependencies.vitest).toBeUndefined();
      expect(devDependencies['vite-plus']).toBe('latest');
    });
  }
});
