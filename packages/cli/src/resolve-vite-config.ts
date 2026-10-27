import fs from 'node:fs';
import path from 'node:path';

import { findMonorepoRoot } from '../binding/index.js';
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
 * Resolve vite.config.ts and return the config object.
 */
export async function resolveViteConfig(cwd: string, options?: ResolveViteConfigOptions) {
  const { resolveConfig, resolveConfigFile } = await import('./index.js');

  // `resolveConfigFile` takes a path, so look up the config here instead of leaving it to Vite.
  // `VITE_CONFIG_FILES` mirrors Vite's lookup order.
  let configFile = findViteConfig(cwd);
  if (!configFile && options?.traverseUp) {
    const workspaceRoot = findMonorepoRoot(cwd);
    if (workspaceRoot) {
      configFile = findViteConfigUp(path.dirname(cwd), workspaceRoot);
    }
  }

  // This loads the config purely to read a non-plugin block (lint/fmt/pack/run/ staged/create…),
  // so `resolveConfigFile` skips the user's plugin factory.
  // `root` resolves from the config's directory, not `cwd`,
  // including for a workspace config found from a package directory by `traverseUp`.
  if (configFile) {
    return resolveConfigFile(configFile);
  }
  // Without a config, callers still expect a resolved config with every block unset.
  // The lookup above already covered Vite's config names,
  // so `configFile: false` skips a second search and runs no user code.
  return resolveConfig({ root: cwd, configFile: false }, 'build');
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
