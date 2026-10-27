import { mkdirSync, mkdtempSync, realpathSync, symlinkSync, writeFileSync } from 'node:fs';
import { rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it } from 'vite-plus/test';

// Import the built core directly: a source-only test cannot detect a missing
// branding patch or a CLI running against an older core artifact.
import { normalizePath, resolveConfig } from '../dist/vite/node/index.js';
import type { InlineConfig, ResolvedConfig } from '../dist/vite/node/index.js';

const resolveWithOptions = resolveConfig as (
  config: InlineConfig,
  command: 'build' | 'serve',
  defaultMode: undefined,
  defaultNodeEnv: undefined,
  isPreview: undefined,
  patchConfig: undefined,
  patchPlugins: undefined,
  internalOptions?: { rootBase?: string },
) => Promise<ResolvedConfig>;

function resolve(config: InlineConfig, rootBase?: string): Promise<ResolvedConfig> {
  return resolveWithOptions(
    config,
    'build',
    undefined,
    undefined,
    undefined,
    undefined,
    undefined,
    rootBase === undefined ? undefined : { rootBase },
  );
}

let tempDir: string;
let configDir: string;

beforeEach(() => {
  tempDir = realpathSync.native(mkdtempSync(path.join(tmpdir(), 'vite-core-config-root-')));
  configDir = path.join(tempDir, 'project');
  mkdirSync(path.join(configDir, 'app'), { recursive: true });
});

afterEach(async () => {
  // Config bundles can remain open briefly on Windows.
  await rm(tempDir, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 });
});

function expectedRoot(root: string): string {
  try {
    return normalizePath(realpathSync.native(root));
  } catch {
    // Vite also permits a root directory that does not exist yet.
    return normalizePath(root);
  }
}

function writeConfig(config: Record<string, unknown>, dir = configDir): string {
  const file = path.join(dir, 'vite.config.mjs');
  writeFileSync(file, `export default ${JSON.stringify(config)};\n`);
  return file;
}

describe('built core config root resolution', () => {
  it('preserves normal Vite root resolution when rootBase is omitted', async () => {
    const cwd = process.cwd();
    for (const root of [undefined, '', 'app']) {
      const configFile = writeConfig({ root });
      const normal = await resolveConfig({ configFile }, 'build');
      const config = await resolve({ configFile });
      expect(config.root).toBe(normal.root);
      expect(config.root).toBe(expectedRoot(path.resolve(cwd, root ?? '')));
    }
    expect(process.cwd()).toBe(cwd);
  });

  it.each([undefined, '', '.', '..', 'app'])(
    'resolves file root %s from the supplied config directory base',
    async (root) => {
      const cwd = process.cwd();
      const configFile = writeConfig({ root });
      const config = await resolve({ configFile }, configDir);

      expect(config.root).toBe(expectedRoot(path.resolve(configDir, root ?? '')));
      expect(config.configFile).toBe(normalizePath(configFile));
      expect(process.cwd()).toBe(cwd);
    },
  );

  it.each([undefined, 'app'])(
    'uses an independent workspace base for root %s in a nested config file',
    async (root) => {
      const nestedDir = path.join(configDir, 'configs', 'nested');
      const workspaceBase = path.join(tempDir, 'workspace');
      mkdirSync(nestedDir, { recursive: true });
      mkdirSync(path.join(workspaceBase, 'app'), { recursive: true });
      const configFile = writeConfig({ root }, nestedDir);
      const config = await resolve({ configFile }, workspaceBase);

      expect(config.root).toBe(expectedRoot(path.resolve(workspaceBase, root ?? '')));
      expect(config.configFile).toBe(normalizePath(configFile));
    },
  );

  it('preserves an absolute root from the config file', async () => {
    const root = path.join(tempDir, 'absolute');
    mkdirSync(root);
    const config = await resolve({ configFile: writeConfig({ root }) }, configDir);

    expect(config.root).toBe(expectedRoot(root));
  });

  it.each(['', 'inline-app', 'absolute'])(
    'keeps explicit inline root precedence and cwd semantics for %s',
    async (value) => {
      const root = value === 'absolute' ? tempDir : value;
      const configFile = writeConfig({ root: 'app' });
      const config = await resolve({ configFile, root }, configDir);

      expect(config.root).toBe(expectedRoot(path.resolve(root)));
    },
  );

  it('leaves no-file resolution unchanged', async () => {
    const config = await resolve({ root: tempDir }, configDir);

    expect(config.configFile).toBeUndefined();
    expect(config.root).toBe(expectedRoot(tempDir));
  });

  it('does not load a file when configFile is false', async () => {
    writeFileSync(path.join(configDir, 'vite.config.mjs'), "throw new Error('must not load');\n");
    for (const root of [undefined, configDir]) {
      const config = await resolve({ configFile: false, root }, configDir);

      expect(config.configFile).toBeUndefined();
      expect(config.root).toBe(expectedRoot(root ?? process.cwd()));
    }
  });

  it('honors a root supplied by a config hook after file root resolution', async () => {
    const configFile = writeConfig({ root: 'app' });
    const config = await resolve(
      {
        configFile,
        plugins: [{ name: 'change-root', config: () => ({ root: 'plugin-app' }) }],
      },
      configDir,
    );

    expect(config.root).toBe(expectedRoot(path.resolve('plugin-app')));
  });

  it.each([false, true])(
    'respects preserveSymlinks=%s for a config loaded through a directory link',
    async (preserveSymlinks) => {
      writeConfig({ root: 'app', resolve: { preserveSymlinks } });
      const link = path.join(tempDir, 'linked-project');
      symlinkSync(configDir, link, process.platform === 'win32' ? 'junction' : 'dir');
      const configFile = path.join(link, 'vite.config.mjs');
      const config = await resolve({ configFile }, link);

      expect(config.configFile).toBe(normalizePath(configFile));
      expect(config.root).toBe(
        preserveSymlinks
          ? normalizePath(path.join(link, 'app'))
          : expectedRoot(path.join(configDir, 'app')),
      );
    },
  );
});
