import fs from 'node:fs';
import path from 'node:path';

import { getEnv } from '@voidzero-dev/vite-task-client';
import type { UserConfig } from 'vite';

import { findMonorepoRoot } from '../binding/index.js';
import { withConfigMetadataResolution } from './define-config.ts';
import { VITE_CONFIG_FILES } from './utils/constants.ts';

/**
 * Find a vite config file by walking up from `startDir` to `stopDir`.
 * Returns the absolute path of the first config file found, or undefined.
 */
export function findViteConfigUp(startDir: string, stopDir: string): string | undefined {
  let dir = path.resolve(startDir);
  const stop = path.resolve(stopDir);
  // Callers may start above `stopDir`, e.g. at the parent of a workspace root.
  const relative = path.relative(stop, dir);
  if (relative === '..' || relative.startsWith(`..${path.sep}`) || path.isAbsolute(relative)) {
    return undefined;
  }

  while (true) {
    for (const filename of VITE_CONFIG_FILES) {
      const filePath = path.join(dir, filename);
      if (fs.existsSync(filePath)) {
        return filePath;
      }
    }
    const parent = path.dirname(dir);
    if (dir === stop || parent === dir) {
      break;
    }
    dir = parent;
  }
  return undefined;
}

/**
 * Find a vite config file directly in `dir` (no walking up). Returns the
 * absolute path of the first config file found, or undefined. Covers every
 * supported extension (`.ts/.js/.mjs/.mts/.cjs/.cts`).
 */
export function findViteConfig(dir: string): string | undefined {
  const filename = VITE_CONFIG_FILES.find((f) => fs.existsSync(path.join(dir, f)));
  return filename ? path.join(dir, filename) : undefined;
}

export function hasViteConfig(dir: string): boolean {
  return findViteConfig(dir) !== undefined;
}

export interface ResolveViteConfigOptions {
  traverseUp?: boolean;
}

/**
 * Load vite.config.ts metadata without resolving Vite's plugins or server options.
 */
export async function resolveViteConfig(
  cwd: string,
  options?: ResolveViteConfigOptions,
): Promise<UserConfig & { configFile: string | undefined }> {
  const { loadConfigFromFile } = await import('./index.js');

  // This loads the config purely to read a non-plugin block (lint/fmt/pack/run/
  // staged/create…), so skip the user's plugin factory while it evaluates.
  return withConfigMetadataResolution(async () => {
    // Preserve resolveConfig's environment defaults, including Vite Task's
    // tracked NODE_ENV, before evaluating function configs and their imports.
    if (process.env.NODE_ENV === undefined) {
      const nodeEnv = getEnv('NODE_ENV');
      if (nodeEnv !== undefined) {
        process.env.NODE_ENV = nodeEnv;
      }
    }
    if (!process.env.NODE_ENV) {
      process.env.NODE_ENV = 'development';
    }

    let configFile: string | undefined;
    if (options?.traverseUp && !hasViteConfig(cwd)) {
      const workspaceRoot = findMonorepoRoot(cwd);
      if (workspaceRoot) {
        configFile = findViteConfigUp(path.dirname(cwd), workspaceRoot);
      }
    }

    const result = await loadConfigFromFile(
      { command: 'build', mode: 'development', isSsrBuild: false, isPreview: false },
      configFile,
      cwd,
    );
    return { ...result?.config, configFile: result?.path };
  });
}

export async function resolveUniversalViteConfig(err: null | Error, viteConfigCwd: string) {
  if (err) {
    throw err;
  }
  try {
    const config = await resolveViteConfig(viteConfigCwd);

    return JSON.stringify({
      configFile: config.configFile,
      lint: config.lint,
      fmt: config.fmt,
      check: config.check,
      run: config.run,
      staged: config.staged,
    });
  } catch (resolveErr) {
    console.error('[Vite+] resolve universal vite config error:', resolveErr);
    throw resolveErr;
  }
}
