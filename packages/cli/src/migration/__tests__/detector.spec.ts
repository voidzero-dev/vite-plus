import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { detectMigrationWarnings } from '../detector.ts';

function writeProject(projectPath: string, packageJson: object, configFiles: string[] = []): void {
  fs.mkdirSync(projectPath, { recursive: true });
  fs.writeFileSync(path.join(projectPath, 'package.json'), JSON.stringify(packageJson));
  for (const configFile of configFiles) {
    fs.writeFileSync(path.join(projectPath, configFile), '');
  }
}

describe('detectMigrationWarnings', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-migration-detector-'));
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  it('detects matching packages and config file variants', () => {
    writeProject(
      tmpDir,
      {
        devDependencies: { rollup: '^4.0.0', jest: '^30.0.0' },
        dependencies: { rolldown: '^1.0.0' },
      },
      ['rollup.config.mjs', 'rolldown.config.cjs', 'jest.config.json'],
    );

    expect(detectMigrationWarnings(tmpDir)).toEqual([
      'Rollup configuration detected. Rollup is API-compatible with Rolldown and can be migrated first before migrating further to tsdown or Vite. Once migrated to tsdown or Vite, re-run vp migrate.',
      'Rolldown configuration detected. Rolldown can be migrated to tsdown or Vite, and it is recommended to do so. Once migrated, re-run vp migrate.',
      'Jest configuration detected. Vitest is highly compatible with Jest, and so we recommend migrating. See the Vitest Jest migration guide: https://vitest.dev/guide/migration/jest, then re-run vp migrate.',
    ]);
  });

  it('requires both a dependency and config for build and test tools', () => {
    writeProject(tmpDir, { devDependencies: { rollup: '^4.0.0' } }, ['jest.config.ts']);

    expect(detectMigrationWarnings(tmpDir)).toEqual([]);
  });

  it('detects Jest configuration in package.json', () => {
    writeProject(tmpDir, {
      devDependencies: { jest: '^30.0.0' },
      jest: { testEnvironment: 'node' },
    });

    expect(detectMigrationWarnings(tmpDir)).toEqual([
      'Jest configuration detected. Vitest is highly compatible with Jest, and so we recommend migrating. See the Vitest Jest migration guide: https://vitest.dev/guide/migration/jest, then re-run vp migrate.',
    ]);
  });

  it('detects matching packages and configs in workspace packages', () => {
    writeProject(path.join(tmpDir, 'packages/app'), { dependencies: { jest: '^30.0.0' } }, [
      'jest.config.ts',
    ]);

    expect(detectMigrationWarnings(tmpDir, [{ name: 'app', path: 'packages/app' }])).toEqual([
      'Jest configuration detected. Vitest is highly compatible with Jest, and so we recommend migrating. See the Vitest Jest migration guide: https://vitest.dev/guide/migration/jest, then re-run vp migrate.',
    ]);
  });

  it('detects workspace configs with dependencies declared at the root', () => {
    writeProject(tmpDir, { devDependencies: { jest: '^30.0.0' } });
    writeProject(path.join(tmpDir, 'packages/app'), {}, ['jest.config.js']);

    expect(detectMigrationWarnings(tmpDir, [{ name: 'app', path: 'packages/app' }])).toEqual([
      'Jest configuration detected. Vitest is highly compatible with Jest, and so we recommend migrating. See the Vitest Jest migration guide: https://vitest.dev/guide/migration/jest, then re-run vp migrate.',
    ]);
  });

  describe('Turborepo', () => {
    const warning =
      'Turborepo detected. Task orchestration and caching were not migrated. vp run does not read Turborepo configuration. See https://viteplus.dev/guide/run and https://viteplus.dev/guide/cache.';

    describe.each(['.', 'packages/app'])('in %s', (packagePath) => {
      const packages = [{ name: 'app', path: 'packages/app' }];

      it.each(['turbo.json', 'turbo.jsonc'])(
        'detects %s without a turbo dependency',
        (configFile) => {
          writeProject(tmpDir, {});
          writeProject(path.join(tmpDir, packagePath), {}, [configFile]);

          expect(detectMigrationWarnings(tmpDir, packages)).toEqual([warning]);
        },
      );

      it.each(['dependencies', 'devDependencies', 'optionalDependencies'])(
        'detects a turbo dependency in %s without a config file',
        (field) => {
          writeProject(tmpDir, {});
          writeProject(path.join(tmpDir, packagePath), { [field]: { turbo: '^2.0.0' } });

          expect(detectMigrationWarnings(tmpDir, packages)).toEqual([warning]);
        },
      );
    });

    it('warns once when the root and multiple packages use Turborepo', () => {
      writeProject(tmpDir, { devDependencies: { turbo: '^2.0.0' } }, ['turbo.json']);
      writeProject(path.join(tmpDir, 'packages/app'), {}, ['turbo.json']);
      writeProject(path.join(tmpDir, 'packages/lib'), { dependencies: { turbo: '^2.0.0' } }, [
        'turbo.jsonc',
      ]);

      expect(
        detectMigrationWarnings(tmpDir, [
          { name: 'app', path: 'packages/app' },
          { name: 'lib', path: 'packages/lib' },
        ]),
      ).toEqual([warning]);
    });

    it('does not warn for related packages without turbo or its config', () => {
      writeProject(tmpDir, { devDependencies: { 'eslint-config-turbo': '^2.0.0' } });
      writeProject(path.join(tmpDir, 'packages/app'), {
        devDependencies: { 'eslint-plugin-turbo': '^2.0.0' },
      });

      expect(detectMigrationWarnings(tmpDir, [{ name: 'app', path: 'packages/app' }])).toEqual([]);
    });
  });
});
