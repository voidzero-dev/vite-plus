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

  it('does not warn when the package and config are not both present', () => {
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
});
