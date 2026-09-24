import fs from 'node:fs';
import path from 'node:path';

import { VITE_CONFIG_FILES } from './utils/constants.ts';

/**
 * Find a vite config file by walking up from `startDir` to `stopDir`.
 * Returns the absolute path of the first config file found, or undefined.
 */
export function findViteConfigUp(startDir: string, stopDir: string): string | undefined {
  let dir = path.resolve(startDir);
  const stop = path.resolve(stopDir);

  while (true) {
    for (const filename of VITE_CONFIG_FILES) {
      const filePath = path.join(dir, filename);
      if (fs.existsSync(filePath)) {
        return filePath;
      }
    }
    const parent = path.dirname(dir);
    if (parent === dir || !parent.startsWith(stop)) {
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
 * Resolve vite.config.ts and return the config object.
 */
export async function resolveViteConfig(cwd: string, options?: ResolveViteConfigOptions) {
  // Discovery helpers and commands without a config must not load Vite/Vitest.
  const [{ resolveConfig }, { withConfigMetadataResolution }] = await Promise.all([
    import('./index.js'),
    import('./define-config.ts'),
  ]);

  // This loads the config purely to read a non-plugin block (lint/fmt/pack/run/
  // staged/create…), so skip the user's plugin factory while it evaluates.
  return withConfigMetadataResolution(async () => {
    if (options?.traverseUp && !hasViteConfig(cwd)) {
      const workspaceRoot = findWorkspaceRoot(cwd);
      if (workspaceRoot) {
        const configFile = findViteConfigUp(path.dirname(cwd), workspaceRoot);
        if (configFile) {
          return resolveConfig({ root: cwd, configFile }, 'build');
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
    // Rust already supplies the workspace root. Without a config there are no
    // user metadata blocks, so there is no need to initialize Vite.
    if (!hasViteConfig(viteConfigCwd)) {
      return '{}';
    }
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
