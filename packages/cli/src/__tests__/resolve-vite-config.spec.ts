import fs from 'node:fs';
import { mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { afterEach, beforeEach, describe, expect, it } from 'vite-plus/test';

import { normalizePath, resolveConfig } from '../index.js';
import {
  findViteConfigUp,
  resolveUniversalViteConfig,
  resolveViteConfig,
} from '../resolve-vite-config.js';

describe('findViteConfigUp', () => {
  let tempDir: string;

  beforeEach(() => {
    // Resolve symlinks (macOS /var -> /private/var) to match path.resolve behavior
    tempDir = fs.realpathSync(mkdtempSync(path.join(tmpdir(), 'vite-config-test-')));
  });

  afterEach(() => {
    fs.rmSync(tempDir, { recursive: true, force: true });
  });

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

describe('resolveViteConfig metadata', () => {
  let tempDir: string;
  let originalNodeEnv: string | undefined;

  beforeEach(() => {
    tempDir = fs.realpathSync(mkdtempSync(path.join(tmpdir(), 'vite-config-metadata-')));
    fs.writeFileSync(path.join(tempDir, 'package.json'), '{"type":"module"}');
    originalNodeEnv = process.env.NODE_ENV;
  });

  afterEach(() => {
    fs.rmSync(tempDir, { recursive: true, force: true });
    if (originalNodeEnv === undefined) {
      delete process.env.NODE_ENV;
    } else {
      process.env.NODE_ENV = originalNodeEnv;
    }
  });

  function writeConfig(source: string): string {
    const configFile = path.join(tempDir, 'vite.config.ts');
    fs.writeFileSync(configFile, source);
    return normalizePath(configFile);
  }

  it('loads tool metadata without running Vite plugin hooks', async () => {
    const configFile = writeConfig(`
      export default {
        run: { tasks: { hello: { command: 'echo hello' } } },
        lint: { rules: { eqeqeq: 'error' } },
        fmt: { semi: false },
        check: { lint: false },
        staged: { '*.ts': 'vp check' },
        pack: { entry: ['src/index.ts'] },
        create: { defaultTemplate: '@example' },
        plugins: [{
          name: 'build-only',
          config() { throw new Error('config hook ran'); },
          configResolved() { throw new Error('configResolved hook ran'); },
        }],
      };
    `);

    const config = await resolveViteConfig(tempDir);
    expect(config).toMatchObject({
      configFile,
      pack: { entry: ['src/index.ts'] },
      create: { defaultTemplate: '@example' },
    });
    expect(JSON.parse(await resolveUniversalViteConfig(null, tempDir))).toEqual({
      configFile,
      run: { tasks: { hello: { command: 'echo hello' } } },
      lint: { rules: { eqeqeq: 'error' } },
      fmt: { semi: false },
      check: { lint: false },
      staged: { '*.ts': 'vp check' },
    });
  }, 15_000);

  it('returns identical metadata across equivalent pnpm installation states', async () => {
    const metadata = { run: { tasks: { hello: { command: 'echo hello' } } }, fmt: {} };
    const configFile = writeConfig(`export default ${JSON.stringify(metadata)};`);
    fs.mkdirSync(path.join(tempDir, 'node_modules'));
    const modulesFile = path.join(tempDir, 'node_modules', '.modules.yaml');
    const results = [];
    for (const install of ['first', 'second']) {
      const storeDir = path.join(tempDir, install);
      const virtualStoreDir = path.join(storeDir, '.pnpm');
      fs.mkdirSync(storeDir);
      // pnpm >= 10.29 writes JSON to this manifest. The lockfile and
      // installed packages are unchanged; only install-local state differs.
      fs.writeFileSync(
        modulesFile,
        JSON.stringify({
          layoutVersion: 5,
          packageManager: 'pnpm@12.11.2',
          prunedAt: install === 'first' ? '2026-09-28T12:55:07Z' : '2026-09-28T12:58:26Z',
          storeDir,
          virtualStoreDir,
        }),
      );
      results.push(await resolveViteConfig(tempDir));
      // The same manifest still affects callers that need full resolution.
      const resolved = await resolveConfig({ root: tempDir }, 'build');
      expect(resolved.server.fs.allow).toContain(normalizePath(virtualStoreDir));
    }

    expect(results[0]).toEqual({ ...metadata, configFile });
    expect(results[1]).toEqual(results[0]);
  });

  it.each([undefined, '', 'production'])(
    'preserves config evaluation with NODE_ENV=%s',
    async (nodeEnv) => {
      if (nodeEnv === undefined) {
        delete process.env.NODE_ENV;
      } else {
        process.env.NODE_ENV = nodeEnv;
      }
      writeConfig(`
      export default async (env) => ({
        run: { tasks: { hello: {
          command: JSON.stringify([
            env.command, env.mode, env.isSsrBuild, env.isPreview, process.env.NODE_ENV,
          ]),
        } } },
      });
    `);

      const config = await resolveViteConfig(tempDir);
      expect(config.run).toEqual({
        tasks: {
          hello: {
            command: JSON.stringify([
              'build',
              'development',
              false,
              false,
              nodeEnv || 'development',
            ]),
          },
        },
      });
    },
  );

  it('leaves full resolution and lazy plugins available after reading metadata', async () => {
    fs.mkdirSync(path.join(tempDir, 'node_modules'));
    fs.symlinkSync(
      fileURLToPath(new URL('../..', import.meta.url)),
      path.join(tempDir, 'node_modules', 'vite-plus'),
      'junction',
    );
    writeConfig(`
      import { writeFileSync } from 'node:fs';
      import { defineConfig, lazyPlugins } from 'vite-plus';
      export default defineConfig({
        fmt: { semi: false },
        plugins: lazyPlugins(() => [{
          name: 'build-only',
          config() { return { define: { __FROM_PLUGIN__: 'true' } }; },
          configResolved(config) {
            writeFileSync(new URL('./resolved.txt', import.meta.url), config.command);
          },
        }]),
      });
    `);

    const metadata = await resolveViteConfig(tempDir);
    expect(metadata.fmt).toEqual({ semi: false });
    expect(fs.existsSync(path.join(tempDir, 'resolved.txt'))).toBe(false);

    const config = await resolveConfig({ root: tempDir }, 'build');
    expect(config.define).toHaveProperty('__FROM_PLUGIN__', 'true');
    expect(config.plugins.some((plugin) => plugin.name === 'build-only')).toBe(true);
    expect(fs.readFileSync(path.join(tempDir, 'resolved.txt'), 'utf8')).toBe('build');
  });

  it('returns no metadata when no config exists', async () => {
    expect(await resolveViteConfig(tempDir)).toEqual({ configFile: undefined });
  });

  it('uses the closest config when traversing within a workspace', async () => {
    fs.writeFileSync(path.join(tempDir, 'pnpm-workspace.yaml'), 'packages: [packages/*]\n');
    const configFile = writeConfig('export default { pack: { entry: ["root.ts"] } };');
    const packageDir = path.join(tempDir, 'packages', 'app');
    fs.mkdirSync(packageDir, { recursive: true });
    fs.writeFileSync(path.join(packageDir, 'package.json'), '{"name":"app","type":"module"}');
    expect(await resolveViteConfig(packageDir)).toEqual({ configFile: undefined });
    expect(await resolveViteConfig(packageDir, { traverseUp: true })).toEqual({
      configFile,
      pack: { entry: ['root.ts'] },
    });

    const localConfig = path.join(packageDir, 'vite.config.mts');
    fs.writeFileSync(localConfig, 'export default { pack: { entry: ["local.ts"] } };');
    expect(await resolveViteConfig(packageDir, { traverseUp: true })).toEqual({
      configFile: normalizePath(localConfig),
      pack: { entry: ['local.ts'] },
    });
  });
});
