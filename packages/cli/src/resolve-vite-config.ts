import fs from 'node:fs';
import path from 'node:path';

const VITE_CONFIG_FILES = [
  'vite.config.ts',
  'vite.config.js',
  'vite.config.mjs',
  'vite.config.mts',
  'vite.config.cjs',
  'vite.config.cts',
];

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

export function hasViteConfig(dir: string): boolean {
  return VITE_CONFIG_FILES.some((f) => fs.existsSync(path.join(dir, f)));
}

/**
 * Find the workspace / repo root by walking up from `startDir`.
 *
 * Monorepo markers (`pnpm-workspace.yaml`, `workspaces` in `package.json`,
 * `lerna.json`) win globally — they always take precedence over any
 * `.git` seen along the way, so a subproject with its own `.git` (git
 * submodule, nested clone) can't preempt a marker higher up the tree.
 * `.git` is only returned when the walk finishes without finding any
 * monorepo marker.
 */
export function findWorkspaceRoot(startDir: string): string | undefined {
  let dir = path.resolve(startDir);
  let gitFallback: string | undefined;
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
    if (gitFallback === undefined && fs.existsSync(path.join(dir, '.git'))) {
      gitFallback = dir;
    }
    const parent = path.dirname(dir);
    if (parent === dir) {
      break;
    }
    dir = parent;
  }
  return gitFallback;
}

export interface ResolveViteConfigOptions {
  traverseUp?: boolean;
}

/**
 * Resolve vite.config.ts and return the config object.
 */
export async function resolveViteConfig(cwd: string, options?: ResolveViteConfigOptions) {
  const { resolveConfig } = await import('./index.js');

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
      run: config.run,
      staged: config.staged,
    });
  } catch (resolveErr) {
    console.error('[Vite+] resolve universal vite config error:', resolveErr);
    throw resolveErr;
  }
}
