import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, describe, expect, it } from 'vitest';

import { prepareNpmViteAliasReinstall } from '../npm-reinstall.ts';

const tempDirs: string[] = [];

function createTempDir(): string {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vite-plus-npm-reinstall-'));
  tempDirs.push(tempDir);
  return tempDir;
}

function writePackage(packagePath: string, name: string): void {
  fs.mkdirSync(packagePath, { recursive: true });
  fs.writeFileSync(path.join(packagePath, 'package.json'), JSON.stringify({ name }));
}

afterEach(() => {
  for (const tempDir of tempDirs.splice(0)) {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
});

describe('prepareNpmViteAliasReinstall', () => {
  it('prunes stale real-Vite lock entries and installations while preserving the core alias', () => {
    const rootDir = createTempDir();
    const staleRootVite = path.join(rootDir, 'node_modules', 'vite');
    const staleNestedVite = path.join(rootDir, 'node_modules', 'consumer', 'node_modules', 'vite');
    const coreVite = path.join(rootDir, 'packages', 'app', 'node_modules', 'vite');
    writePackage(staleRootVite, 'vite');
    writePackage(staleNestedVite, 'vite');
    writePackage(coreVite, '@voidzero-dev/vite-plus-core');
    fs.writeFileSync(
      path.join(rootDir, 'package-lock.json'),
      JSON.stringify({
        lockfileVersion: 3,
        packages: {
          '': { name: 'test' },
          'node_modules/vite': {
            version: '7.3.5',
            resolved: 'https://registry.npmjs.org/vite/-/vite-7.3.5.tgz',
          },
          'node_modules/consumer/node_modules/vite': {
            version: '7.3.5',
            resolved: 'https://registry.npmjs.org/vite/-/vite-7.3.5.tgz',
          },
          'packages/app/node_modules/vite': {
            name: '@voidzero-dev/vite-plus-core',
            version: '0.2.1',
          },
        },
      }),
    );

    expect(
      prepareNpmViteAliasReinstall(rootDir, [rootDir, path.join(rootDir, 'packages', 'app')])
        .changed,
    ).toBe(true);

    const lock = JSON.parse(fs.readFileSync(path.join(rootDir, 'package-lock.json'), 'utf8')) as {
      packages: Record<string, unknown>;
    };
    expect(lock.packages['node_modules/vite']).toBeUndefined();
    expect(lock.packages['node_modules/consumer/node_modules/vite']).toBeUndefined();
    expect(lock.packages['packages/app/node_modules/vite']).toBeDefined();
    expect(fs.existsSync(staleRootVite)).toBe(false);
    expect(fs.existsSync(staleNestedVite)).toBe(false);
    expect(fs.existsSync(coreVite)).toBe(true);
  });

  it('removes a stale workspace-local install when no package-lock exists', () => {
    const rootDir = createTempDir();
    const workspaceDir = path.join(rootDir, 'packages', 'app');
    const staleVite = path.join(workspaceDir, 'node_modules', 'vite');
    writePackage(staleVite, 'vite');

    expect(prepareNpmViteAliasReinstall(rootDir, [rootDir, workspaceDir]).changed).toBe(true);
    expect(fs.existsSync(staleVite)).toBe(false);
    expect(prepareNpmViteAliasReinstall(rootDir, [rootDir, workspaceDir]).changed).toBe(false);
  });

  it('restores the moved-aside Vite install when the reinstall fails and deletes it on commit', () => {
    const rootDir = createTempDir();
    const staleVite = path.join(rootDir, 'node_modules', 'vite');
    writePackage(staleVite, 'vite');

    const preparation = prepareNpmViteAliasReinstall(rootDir);
    expect(preparation.changed).toBe(true);
    expect(fs.existsSync(staleVite)).toBe(false);

    // Install failed: the previously working Vite comes back.
    preparation.restore();
    expect(fs.existsSync(path.join(staleVite, 'package.json'))).toBe(true);

    const secondPreparation = prepareNpmViteAliasReinstall(rootDir);
    expect(secondPreparation.changed).toBe(true);
    // Install succeeded: the backup is deleted for good.
    secondPreparation.commit();
    expect(fs.existsSync(staleVite)).toBe(false);
    expect(fs.readdirSync(path.join(rootDir, 'node_modules'))).toEqual([]);
  });

  it('does not throw on a malformed package-lock.json and still prunes install trees', () => {
    const rootDir = createTempDir();
    const workspaceDir = path.join(rootDir, 'packages', 'app');
    const staleVite = path.join(workspaceDir, 'node_modules', 'vite');
    writePackage(staleVite, 'vite');
    // A merge-conflicted / truncated lockfile (e.g. an interrupted prior
    // `npm install`) must not abort the migration with an uncaught SyntaxError.
    fs.writeFileSync(
      path.join(rootDir, 'package-lock.json'),
      '<<<<<<< HEAD\n{ "lockfileVersion": 3 }\n=======\n{}\n>>>>>>> incoming\n',
    );

    expect(() => prepareNpmViteAliasReinstall(rootDir, [rootDir, workspaceDir])).not.toThrow();
    expect(fs.existsSync(staleVite)).toBe(false);
  });
});
