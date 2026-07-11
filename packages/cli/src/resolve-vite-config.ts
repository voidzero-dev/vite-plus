import fs from 'node:fs';
import path from 'node:path';

import { withConfigMetadataResolution } from './define-config.ts';
import {
  findSupportedConfigFile,
  findSupportedConfigFileUp,
  hasSupportedConfigFile,
} from './utils/config-files.ts';

/**
 * Find a supported config file by walking up from `startDir` to `stopDir`.
 * Returns the absolute path of the first config file found, or undefined.
 */
export function findViteConfigUp(startDir: string, stopDir: string): string | undefined {
  return findSupportedConfigFileUp(startDir, stopDir);
}

/**
 * Find a supported config file directly in `dir` (no walking up). Returns the
 * absolute path of the first config file found, or undefined.
 */
export function findViteConfig(dir: string): string | undefined {
  return findSupportedConfigFile(dir);
}

export function hasViteConfig(dir: string): boolean {
  return hasSupportedConfigFile(dir);
}

/**
 * Find the workspace root by walking up from `startDir` looking for
 * monorepo indicators (pnpm-workspace.yaml, workspaces in package.json, lerna.json).
 */
export function findWorkspaceRoot(startDir: string): string | undefined {
  let dir = path.resolve(startDir);
  while (true) {
    if (fs.existsSync(path.join(dir, 'pnpm-workspace.yaml'))) {
      return dir;
    }
    const pkgPath = path.join(dir, 'package.json');
    if (fs.existsSync(pkgPath)) {
      try {
        const pkg = JSON.parse(fs.readFileSync(pkgPath, 'utf-8'));
        if (pkg.workspaces) {
          return dir;
        }
      } catch {
        // Skip malformed package.json and continue searching parent directories
      }
    }
    if (fs.existsSync(path.join(dir, 'lerna.json'))) {
      return dir;
    }
    const parent = path.dirname(dir);
    if (parent === dir) {
      break;
    }
    dir = parent;
  }
  return undefined;
}

export interface ResolveViteConfigOptions {
  traverseUp?: boolean;
}

/**
 * Resolve the project's supported config file and return the config object.
 */
export async function resolveViteConfig(cwd: string, options?: ResolveViteConfigOptions) {
  const { resolveConfig } = await import('./index.js');

  // This loads the config purely to read a non-plugin block (lint/fmt/pack/run/
  // staged/create…), so skip the user's plugin factory while it evaluates.
  return withConfigMetadataResolution(async () => {
    const configFile = findViteConfig(cwd);
    if (configFile) {
      return resolveConfig({ root: cwd, configFile }, 'build');
    }

    if (options?.traverseUp) {
      const workspaceRoot = findWorkspaceRoot(cwd);
      if (workspaceRoot) {
        const upConfigFile = findViteConfigUp(path.dirname(cwd), workspaceRoot);
        if (upConfigFile) {
          return resolveConfig({ root: cwd, configFile: upConfigFile }, 'build');
        }
      }
    }

    return resolveConfig({ root: cwd }, 'build');
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
