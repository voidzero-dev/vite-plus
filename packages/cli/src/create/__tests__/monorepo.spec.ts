import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { PackageManager } from '../../types/index.js';
import { dropAliasedRuntimeDevDeps } from '../templates/monorepo.js';

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
