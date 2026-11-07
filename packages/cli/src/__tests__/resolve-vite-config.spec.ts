import fs from 'node:fs';
import { mkdtempSync } from 'node:fs';
import { rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';

import { normalizePath } from 'vite';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { findViteConfigUp, resolveViteConfig } from '../resolve-vite-config.js';

let tempDir: string;

beforeEach(() => {
  // Resolve symlinks (macOS /var -> /private/var) to match path.resolve behavior
  tempDir = fs.realpathSync(mkdtempSync(path.join(tmpdir(), 'vite-config-test-')));
});

afterEach(async () => {
  // Vite removes its bundled config asynchronously. Allow transient Windows
  // file locks to clear; persistent cleanup errors still fail the test.
  await rm(tempDir, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 });
});

describe('findViteConfigUp', () => {
  it('should find config in the start directory', () => {
    fs.writeFileSync(path.join(tempDir, 'vite.config.ts'), '');
    const result = findViteConfigUp(tempDir, tempDir);
    expect(result).toBe(path.join(tempDir, 'vite.config.ts'));
  });

  it('should find config in a parent directory', () => {
    const subDir = path.join(tempDir, 'packages', 'my-lib');
    fs.mkdirSync(subDir, { recursive: true });
    fs.writeFileSync(path.join(tempDir, 'vite.config.ts'), '');

    const result = findViteConfigUp(subDir, tempDir);
    expect(result).toBe(path.join(tempDir, 'vite.config.ts'));
  });

  it('should find config in an intermediate directory', () => {
    const subDir = path.join(tempDir, 'packages', 'my-lib', 'src');
    fs.mkdirSync(subDir, { recursive: true });
    fs.writeFileSync(path.join(tempDir, 'packages', 'vite.config.ts'), '');

    const result = findViteConfigUp(subDir, tempDir);
    expect(result).toBe(path.join(tempDir, 'packages', 'vite.config.ts'));
  });

  it('should return undefined when no config exists', () => {
    const subDir = path.join(tempDir, 'packages', 'my-lib');
    fs.mkdirSync(subDir, { recursive: true });

    const result = findViteConfigUp(subDir, tempDir);
    expect(result).toBeUndefined();
  });

  it('should not traverse beyond stopDir', () => {
    const parentConfig = path.join(tempDir, 'vite.config.ts');
    fs.writeFileSync(parentConfig, '');
    const stopDir = path.join(tempDir, 'packages');
    const subDir = path.join(stopDir, 'my-lib');
    fs.mkdirSync(subDir, { recursive: true });

    const result = findViteConfigUp(subDir, stopDir);
    // Should not find the config in tempDir because stopDir is packages/
    expect(result).toBeUndefined();
  });

  it('should not search above stopDir when starting from its parent', () => {
    fs.writeFileSync(path.join(tempDir, 'vite.config.ts'), '');
    const stopDir = path.join(tempDir, 'monorepo');
    fs.mkdirSync(stopDir);

    const result = findViteConfigUp(tempDir, stopDir);
    expect(result).toBeUndefined();
  });

  it('should prefer the closest config file', () => {
    const subDir = path.join(tempDir, 'packages', 'my-lib');
    fs.mkdirSync(subDir, { recursive: true });
    fs.writeFileSync(path.join(tempDir, 'vite.config.ts'), '');
    fs.writeFileSync(path.join(tempDir, 'packages', 'vite.config.ts'), '');

    const result = findViteConfigUp(subDir, tempDir);
    expect(result).toBe(path.join(tempDir, 'packages', 'vite.config.ts'));
  });

  it.each(['js', 'mts', 'cjs', 'cts', 'mjs'])('should find .%s config files', (extension) => {
    const subDir = path.join(tempDir, 'packages', 'my-lib');
    fs.mkdirSync(subDir, { recursive: true });
    fs.writeFileSync(path.join(tempDir, `vite.config.${extension}`), '');

    const result = findViteConfigUp(subDir, tempDir);
    expect(result).toBe(path.join(tempDir, `vite.config.${extension}`));
  });
});

// Plugins read `root` in their `config` hook, as TanStack Start does to find
// its entries, so the fixture plugin records the value it sees there.
function writeConfig(dir: string, root?: string): string {
  const configFile = path.join(dir, 'vite.config.mjs');
  const rootField = root === undefined ? '' : `root: ${JSON.stringify(root)},`;
  fs.writeFileSync(
    configFile,
    `export default {
  ${rootField}
  plugins: [{ name: 'record-root', config: (config) => ({ seenRoot: config.root }) }],
};
`,
  );
  return configFile;
}

// Vite stores the real path, which may differ from `tempDir` in casing or
// short names on Windows.
function realPath(file: string): string {
  return fs.realpathSync.native(file);
}

describe('resolveViteConfig', () => {
  it.each(['unset', 'relative', 'absolute'] as const)(
    'uses the caller root when the local config root is %s',
    async (kind) => {
      const root = {
        unset: undefined,
        relative: 'app',
        absolute: path.join(tempDir, 'app'),
      }[kind];
      const configFile = writeConfig(tempDir, root);

      const config = await resolveViteConfig(tempDir);

      expect(config.configFile).toBe(normalizePath(configFile));
      expect(config).toHaveProperty('seenRoot', tempDir);
      expect(realPath(config.root)).toBe(realPath(tempDir));
      expect(config.inlineConfig.root).toBe(tempDir);
    },
  );

  it.each(['unset', 'relative', 'absolute'] as const)(
    'uses the package root when traverseUp finds a config with root %s',
    async (kind) => {
      fs.writeFileSync(path.join(tempDir, 'pnpm-workspace.yaml'), "packages:\n  - 'packages/*'\n");
      fs.writeFileSync(path.join(tempDir, 'package.json'), '{"name":"fixture"}');
      const root = {
        unset: undefined,
        relative: 'app',
        absolute: path.join(tempDir, 'app'),
      }[kind];
      const configFile = writeConfig(tempDir, root);
      const packageDir = path.join(tempDir, 'packages', 'lib');
      fs.mkdirSync(packageDir, { recursive: true });
      fs.writeFileSync(path.join(packageDir, 'package.json'), '{"name":"lib"}');

      const config = await resolveViteConfig(packageDir, { traverseUp: true });

      expect(config.configFile).toBe(normalizePath(configFile));
      expect(config).toHaveProperty('seenRoot', packageDir);
      expect(realPath(config.root)).toBe(realPath(packageDir));
      expect(config.inlineConfig.root).toBe(packageDir);
    },
  );

  it('resolves without a config file', async () => {
    const config = await resolveViteConfig(tempDir);

    expect(config.configFile).toBeUndefined();
    expect(config.lint).toBeUndefined();
    expect(realPath(config.root)).toBe(realPath(tempDir));
    expect(config.inlineConfig.root).toBe(tempDir);
  });
});
