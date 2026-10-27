import { execFile } from 'node:child_process';
import fs from 'node:fs';
import { mkdtempSync } from 'node:fs';
import { rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

import { normalizePath, resolveConfig } from 'vite';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { resolveConfigFromFile } from '../index.js';
import { findViteConfigUp, resolveViteConfig } from '../resolve-vite-config.js';

let tempDir: string;
const exec = promisify(execFile);

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
function writeConfig(dir: string, fields = ''): string {
  const configFile = path.join(dir, 'vite.config.mjs');
  fs.writeFileSync(
    configFile,
    `export default {
  ${fields}
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

describe('resolveConfigFromFile', () => {
  it('rejects an empty path instead of searching cwd', async () => {
    await expect(resolveConfigFromFile('')).rejects.toThrow('Config file path must not be empty.');
  });

  it.each(['unset', 'empty', 'relative', 'absolute'])(
    'preserves normal Vite root resolution when root is %s and rootBase is omitted',
    async (kind) => {
      const root =
        kind === 'unset'
          ? undefined
          : kind === 'empty'
            ? ''
            : kind === 'relative'
              ? 'app'
              : tempDir;
      const configFile = writeConfig(
        tempDir,
        root === undefined ? '' : `root: ${JSON.stringify(root)},`,
      );
      const cwd = process.cwd();

      const expected = await resolveConfig({ configFile }, 'build');
      const config = await resolveConfigFromFile(configFile);

      expect(config.root).toBe(expected.root);
      if (root === undefined) {
        expect(config).not.toHaveProperty('seenRoot');
      } else {
        expect(config).toHaveProperty('seenRoot', root);
      }
      expect(config.inlineConfig.root).toBeUndefined();
      expect(process.cwd()).toBe(cwd);
    },
  );

  it('uses an explicit workspace base when the config is in a subdirectory', async () => {
    const configDir = path.join(tempDir, 'config');
    fs.mkdirSync(configDir);
    const configFile = writeConfig(configDir);

    const config = await resolveConfigFromFile(configFile, { rootBase: tempDir });

    expect(config).toHaveProperty('seenRoot', tempDir);
    expect(realPath(config.root)).toBe(realPath(tempDir));
  });

  it('resolves a relative root from the workspace base instead of the config directory', async () => {
    const configDir = path.join(tempDir, 'config');
    fs.mkdirSync(configDir);
    const configFile = writeConfig(configDir, "root: 'app',");
    fs.mkdirSync(path.join(tempDir, 'app'));

    const config = await resolveConfigFromFile(configFile, { rootBase: tempDir });

    expect(config).toHaveProperty('seenRoot', path.join(tempDir, 'app'));
    expect(realPath(config.root)).toBe(realPath(path.join(tempDir, 'app')));
  });

  it('preserves an absolute config root when a workspace base is supplied', async () => {
    const root = path.join(tempDir, 'app');
    const rootBase = path.join(tempDir, 'workspace');
    fs.mkdirSync(root);
    fs.mkdirSync(rootBase);
    const configFile = writeConfig(tempDir, `root: ${JSON.stringify(root)},`);

    const config = await resolveConfigFromFile(configFile, { rootBase });

    expect(config).toHaveProperty('seenRoot', root);
    expect(realPath(config.root)).toBe(realPath(root));
  });

  it('keeps the config file and its dependencies', async () => {
    const configFile = path.join(tempDir, 'vite.config.mjs');
    fs.writeFileSync(path.join(tempDir, 'lint.mjs'), 'export const lint = { rules: {} };\n');
    fs.writeFileSync(configFile, "import { lint } from './lint.mjs';\nexport default { lint };\n");

    const config = await resolveConfigFromFile(configFile);

    expect(config.lint).toEqual({ rules: {} });
    expect(config.configFile).toBe(normalizePath(configFile));
    expect(config.configFileDependencies.map(realPath)).toContain(
      realPath(path.join(tempDir, 'lint.mjs')),
    );
  });

  it('provides the root before plugin filtering and early config hooks', async () => {
    const configFile = path.join(tempDir, 'vite.config.mjs');
    const root = path.join(tempDir, 'app');
    fs.mkdirSync(root);
    fs.writeFileSync(
      configFile,
      `import assert from 'node:assert/strict';
export default {
  root: 'app',
  plugins: [{
    name: 'check-early-root',
    apply(config) {
      assert.equal(config.root, ${JSON.stringify(root)});
      return true;
    },
    config: {
      order: 'pre',
      handler(config) {
        assert.equal(config.root, ${JSON.stringify(root)});
        return { lint: { rules: { 'no-console': 'error' } } };
      },
    },
  }],
};\n`,
    );
    const cwd = process.cwd();

    const config = await resolveConfigFromFile(configFile, { rootBase: tempDir });

    expect(config.lint).toEqual({ rules: { 'no-console': 'error' } });
    expect(process.cwd()).toBe(cwd);
  });

  it('provides file metadata to sync and async hooks and retains added dependencies', async () => {
    const configFile = path.join(tempDir, 'vite.config.mjs');
    const dependency = path.join(tempDir, 'lint.mjs');
    const pluginDependency = path.join(tempDir, 'plugin-settings.json');
    fs.writeFileSync(dependency, 'export const lint = { rules: {} };\n');
    fs.writeFileSync(pluginDependency, '{}');
    fs.writeFileSync(
      configFile,
      `import assert from 'node:assert/strict';
import { realpathSync } from 'node:fs';
import { lint } from './lint.mjs';
function checkMetadata(config) {
  assert.equal(config.configFile, ${JSON.stringify(normalizePath(configFile))});
  assert.ok(config.configFileDependencies.map((file) => realpathSync.native(file)).includes(
    realpathSync.native(${JSON.stringify(dependency)}),
  ));
  assert.equal(process.env.VP_RESOLVING_CONFIG_METADATA, '1');
}
export default {
  lint,
  plugins: [{
    name: 'check-sync-metadata',
    configResolved: {
      order: 'pre',
      handler(config) {
        checkMetadata(config);
        config.configFileDependencies.push(${JSON.stringify(normalizePath(pluginDependency))});
      },
    },
  }, {
    name: 'check-async-metadata',
    async configResolved(config) {
      await Promise.resolve();
      checkMetadata(config);
    },
  }],
};\n`,
    );

    const config = await resolveConfigFromFile(configFile);

    expect(config.configFileDependencies.map(realPath)).toContain(realPath(pluginDependency));
  });

  it('preserves Vite config environments and keeps the file config out of inlineConfig', async () => {
    const configFile = path.join(tempDir, 'vite.config.mjs');
    fs.writeFileSync(
      configFile,
      `export default (env) => ({
  mode: 'custom',
  build: { ssr: true },
  functionEnv: { ...env },
  lint: { rules: {} },
  plugins: [{
    name: 'record-environment',
    config(_config, hookEnv) { return { hookEnv: { ...hookEnv } }; },
  }],
});\n`,
    );

    const config = await resolveConfigFromFile(configFile, { rootBase: tempDir });

    expect(config).toHaveProperty('functionEnv', {
      command: 'build',
      mode: 'development',
      isSsrBuild: false,
      isPreview: false,
    });
    expect(config).toHaveProperty('hookEnv', {
      command: 'build',
      mode: 'custom',
      isSsrBuild: false,
      isPreview: false,
    });
    expect(config.mode).toBe('custom');
    expect(config.inlineConfig.configFile).toBe(configFile);
    expect(config.inlineConfig.root).toBeUndefined();
    expect(config.inlineConfig.lint).toBeUndefined();
    expect(config.inlineConfig.build?.ssr).toBeUndefined();
  });

  it.each(['object', 'function', 'async function'])(
    'evaluates a %s config only once',
    async (kind) => {
      const configFile = path.join(tempDir, 'vite.config.mjs');
      const evaluationsFile = path.join(tempDir, 'evaluations.log');
      const exportValue =
        kind === 'object'
          ? '{ lint: { rules: {} } }'
          : `${kind === 'async function' ? 'async ' : ''}() => {
  appendFileSync(${JSON.stringify(evaluationsFile)}, 'function\\n');
  return { lint: { rules: {} } };
}`;
      fs.writeFileSync(
        configFile,
        `import { appendFileSync } from 'node:fs';
appendFileSync(${JSON.stringify(evaluationsFile)}, 'module\\n');
export default ${exportValue};\n`,
      );

      await resolveConfigFromFile(configFile);

      expect(fs.readFileSync(evaluationsFile, 'utf8')).toBe(
        kind === 'object' ? 'module\n' : 'module\nfunction\n',
      );
    },
  );

  it.each([
    { name: 'unset', nodeEnv: undefined, taskNodeEnv: undefined, expected: 'development' },
    { name: 'explicit', nodeEnv: 'production', taskNodeEnv: undefined, expected: 'production' },
    {
      name: 'provided by Vite Task',
      nodeEnv: undefined,
      taskNodeEnv: 'production',
      expected: 'production',
    },
  ])(
    'initializes NODE_ENV before evaluation when $name',
    async ({ nodeEnv, taskNodeEnv, expected }) => {
      const configFile = path.join(tempDir, 'vite.config.mjs');
      fs.writeFileSync(configFile, 'export default () => ({ nodeEnv: process.env.NODE_ENV });\n');
      const env = { ...process.env };
      delete env.NODE_ENV;
      delete env.VP_RUN_NODE_CLIENT_PATH;
      if (nodeEnv !== undefined) {
        env.NODE_ENV = nodeEnv;
      }
      if (taskNodeEnv !== undefined) {
        const clientFile = path.join(tempDir, 'task-client.cjs');
        fs.writeFileSync(
          clientFile,
          `exports.load = () => ({
  getEnv: (name) => name === 'NODE_ENV' ? ${JSON.stringify(taskNodeEnv)} : undefined,
  getEnvs: () => ({}),
  ignoreInput() {},
  ignoreOutput() {},
});\n`,
        );
        env.VP_RUN_NODE_CLIENT_PATH = clientFile;
      }

      // Start outside Vitest's NODE_ENV=test and cached Vite Task client state.
      await exec(
        process.execPath,
        [
          '--input-type=module',
          '--eval',
          `import assert from 'node:assert/strict';
import { resolveConfigFromFile } from ${JSON.stringify(new URL('../define-config.ts', import.meta.url).href)};
const first = await resolveConfigFromFile(${JSON.stringify(configFile)});
const second = await resolveConfigFromFile(${JSON.stringify(configFile)});
assert.equal(first.nodeEnv, ${JSON.stringify(expected)});
assert.equal(second.nodeEnv, ${JSON.stringify(expected)});`,
        ],
        { env, cwd: tempDir },
      );
    },
  );

  it('evaluates the config as a config-metadata resolution', async () => {
    const configFile = path.join(tempDir, 'vite.config.mjs');
    fs.writeFileSync(
      configFile,
      'export default () => ({ metadata: process.env.VP_RESOLVING_CONFIG_METADATA });\n',
    );

    const config = await resolveConfigFromFile(configFile);

    expect(config).toHaveProperty('metadata', '1');
  });

  it('restores the metadata marker when a plugin rejects', async () => {
    const originalMetadata = process.env.VP_RESOLVING_CONFIG_METADATA;
    const configFile = path.join(tempDir, 'vite.config.mjs');
    fs.writeFileSync(
      configFile,
      `export default {
  plugins: [{ name: 'reject-config', configResolved() { throw new Error('plugin rejected'); } }],
};\n`,
    );

    await expect(resolveConfigFromFile(configFile)).rejects.toThrow('plugin rejected');

    expect(process.env.VP_RESOLVING_CONFIG_METADATA).toBe(originalMetadata);
  });

  it.each(['import', 'require'])('resolves through the built %s entry', async (entryType) => {
    const launcher = path.join(tempDir, 'launcher');
    const workspace = path.join(tempDir, 'workspace');
    const configFile = path.join(workspace, 'config', 'vite.config.mjs');
    fs.mkdirSync(path.dirname(configFile), { recursive: true });
    fs.mkdirSync(path.join(launcher, 'app'), { recursive: true });
    fs.mkdirSync(path.join(workspace, 'app'));
    const esmEntry = new URL('../../dist/index.js', import.meta.url);
    const entry = new URL(
      entryType === 'import' ? '../../dist/index.js' : '../../dist/index.cjs',
      import.meta.url,
    );
    fs.writeFileSync(
      configFile,
      `import assert from 'node:assert/strict';
import { defineConfig, lazyPlugins } from ${JSON.stringify(normalizePath(fileURLToPath(esmEntry)))};
export default defineConfig({
  root: 'app',
  lint: { rules: {} },
  plugins: [
    lazyPlugins(() => { throw new Error('lazy plugin factory must not run'); }),
    {
      name: 'check-published-helper',
      config(config) { return { seenRoot: config.root }; },
      configResolved(config) {
        assert.equal(config.configFile, ${JSON.stringify(normalizePath(configFile))});
        assert.equal(process.env.VP_RESOLVING_CONFIG_METADATA, '1');
      },
    },
  ],
});\n`,
    );

    await exec(
      process.execPath,
      [
        '--input-type=module',
        '--eval',
        `import assert from 'node:assert/strict';
import { realpathSync } from 'node:fs';
import { createRequire } from 'node:module';
const api = ${
          entryType === 'import'
            ? `await import(${JSON.stringify(entry.href)})`
            : `createRequire(import.meta.url)(${JSON.stringify(fileURLToPath(entry))})`
        };
const marker = process.env.VP_RESOLVING_CONFIG_METADATA;
const cwd = process.cwd();
const configFile = ${JSON.stringify(path.relative(launcher, configFile))};
const defaults = await api.resolveConfigFromFile(configFile);
assert.equal(realpathSync.native(defaults.root), realpathSync.native(${JSON.stringify(path.join(launcher, 'app'))}));
assert.equal(defaults.seenRoot, 'app');
assert.equal(process.env.VP_RESOLVING_CONFIG_METADATA, marker);
const config = await api.resolveConfigFromFile(configFile, { rootBase: ${JSON.stringify(path.relative(launcher, workspace))} });
assert.equal(realpathSync.native(config.root), realpathSync.native(${JSON.stringify(path.join(workspace, 'app'))}));
assert.equal(config.seenRoot, ${JSON.stringify(path.join(workspace, 'app'))});
assert.deepEqual(config.lint, { rules: {} });
assert.equal(config.inlineConfig.root, undefined);
assert.equal(process.cwd(), cwd);
assert.equal(process.env.VP_RESOLVING_CONFIG_METADATA, marker);`,
      ],
      { cwd: launcher },
    );
  });
});

describe('resolveViteConfig', () => {
  it.each(['unset', 'relative', 'absolute'])(
    'uses the caller root when the local config root is %s',
    async (kind) => {
      const root = kind === 'relative' ? 'app' : path.join(tempDir, 'app');
      const configFile = writeConfig(
        tempDir,
        kind === 'unset' ? '' : `root: ${JSON.stringify(root)},`,
      );

      const config = await resolveViteConfig(tempDir);

      expect(config.configFile).toBe(normalizePath(configFile));
      expect(config).toHaveProperty('seenRoot', tempDir);
      expect(realPath(config.root)).toBe(realPath(tempDir));
      expect(config.inlineConfig.root).toBe(tempDir);
    },
  );

  it.each(['unset', 'relative', 'absolute'])(
    'uses the package root when traverseUp finds a config with root %s',
    async (kind) => {
      fs.writeFileSync(path.join(tempDir, 'pnpm-workspace.yaml'), "packages:\n  - 'packages/*'\n");
      fs.writeFileSync(path.join(tempDir, 'package.json'), '{"name":"fixture"}');
      const root = kind === 'relative' ? 'app' : path.join(tempDir, 'app');
      const configFile = writeConfig(
        tempDir,
        kind === 'unset' ? '' : `root: ${JSON.stringify(root)},`,
      );
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
