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

function hasViteConfig(dir: string): boolean {
  return VITE_CONFIG_FILES.some((f) => fs.existsSync(path.join(dir, f)));
}

/**
 * Find the workspace root by walking up from `startDir` looking for
 * monorepo indicators (pnpm-workspace.yaml, workspaces in package.json, lerna.json).
 */
function findWorkspaceRoot(startDir: string): string | undefined {
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

/**
 * Merge two lint configs. The cwd config overrides the root config.
 * - plugins: union (deduplicated)
 * - rules: shallow merge (cwd wins)
 * - ignorePatterns: cwd overrides entirely
 * - options: shallow merge (cwd wins)
 * - overrides: concatenated
 * - other fields: cwd wins if present
 */
export function mergeLintConfig(
  rootLint: Record<string, unknown> | undefined,
  cwdLint: Record<string, unknown> | undefined,
): Record<string, unknown> | undefined {
  if (!cwdLint) {
    return rootLint;
  }
  if (!rootLint) {
    return cwdLint;
  }

  const merged: Record<string, unknown> = { ...rootLint, ...cwdLint };

  // plugins: union with dedup
  const rootPlugins = (rootLint.plugins as string[]) ?? [];
  const cwdPlugins = (cwdLint.plugins as string[]) ?? [];
  if (rootPlugins.length > 0 || cwdPlugins.length > 0) {
    merged.plugins = [...new Set([...rootPlugins, ...cwdPlugins])];
  }

  // rules: shallow merge
  if (rootLint.rules || cwdLint.rules) {
    merged.rules = {
      ...(rootLint.rules as Record<string, unknown>),
      ...(cwdLint.rules as Record<string, unknown>),
    };
  }

  // options: shallow merge
  if (rootLint.options || cwdLint.options) {
    merged.options = {
      ...(rootLint.options as Record<string, unknown>),
      ...(cwdLint.options as Record<string, unknown>),
    };
  }

  // overrides: concatenate
  const rootOverrides = (rootLint.overrides as unknown[]) ?? [];
  const cwdOverrides = (cwdLint.overrides as unknown[]) ?? [];
  if (rootOverrides.length > 0 || cwdOverrides.length > 0) {
    merged.overrides = [...rootOverrides, ...cwdOverrides];
  }

  return merged;
}

/**
 * Write merged lint config to a fixed path in node_modules/.cache/vite-plus/.
 * Using a fixed path ensures vite-task's cache key (which includes CLI args)
 * stays stable across runs.
 */
function writeMergedLintConfig(cwd: string, mergedLint: Record<string, unknown>): string {
  const cacheDir = path.join(cwd, 'node_modules', '.cache', 'vite-plus');
  fs.mkdirSync(cacheDir, { recursive: true });
  const configPath = path.join(cacheDir, 'merged-lint-config.json');
  fs.writeFileSync(configPath, JSON.stringify(mergedLint, null, 2));
  return configPath;
}

/**
 * Resolve vite config for lint/fmt/staged commands.
 *
 * The argument can be either:
 * - A plain string (workspace path) for backward compatibility
 * - A JSON string `{"workspacePath": "...", "cwd": "..."}` when cwd differs
 *   from workspace root (e.g., running `vp lint` in a sub-package)
 *
 * When cwd differs from workspacePath:
 * 1. Resolve root config (from workspacePath)
 * 2. Resolve cwd config (from cwd)
 * 3. Merge lint configs (root as base, cwd overrides)
 * 4. Write merged lint to a fixed cache file
 * 5. Return the cache file as configFile
 */
export async function resolveUniversalViteConfig(err: null | Error, arg: string) {
  if (err) {
    throw err;
  }
  try {
    let workspacePath: string;
    let cwd: string | undefined;

    // Parse argument: plain string or JSON with workspacePath + cwd
    if (arg.startsWith('{')) {
      const parsed = JSON.parse(arg) as { workspacePath: string; cwd: string };
      workspacePath = parsed.workspacePath;
      cwd = parsed.cwd;
    } else {
      workspacePath = arg;
    }

    const rootConfig = await resolveViteConfig(workspacePath);

    // If cwd is different from workspace root and has its own vite config,
    // merge lint configs
    if (cwd && cwd !== workspacePath && hasViteConfig(cwd)) {
      const cwdConfig = await resolveViteConfig(cwd);
      const mergedLint = mergeLintConfig(
        rootConfig.lint as Record<string, unknown> | undefined,
        cwdConfig.lint as Record<string, unknown> | undefined,
      );
      const mergedFmt = cwdConfig.fmt ?? rootConfig.fmt;

      const configFile = cwdConfig.configFile ?? rootConfig.configFile;

      if (mergedLint) {
        const mergedConfigPath = writeMergedLintConfig(cwd, mergedLint);
        return JSON.stringify({
          configFile,
          lintConfigFile: mergedConfigPath,
          lint: mergedLint,
          fmt: mergedFmt,
          run: cwdConfig.run ?? rootConfig.run,
          staged: cwdConfig.staged ?? rootConfig.staged,
        });
      }

      return JSON.stringify({
        configFile,
        lint: cwdConfig.lint ?? rootConfig.lint,
        fmt: mergedFmt,
        run: cwdConfig.run ?? rootConfig.run,
        staged: cwdConfig.staged ?? rootConfig.staged,
      });
    }

    return JSON.stringify({
      configFile: rootConfig.configFile,
      lint: rootConfig.lint,
      fmt: rootConfig.fmt,
      run: rootConfig.run,
      staged: rootConfig.staged,
    });
  } catch (resolveErr) {
    console.error('[Vite+] resolve universal vite config error:', resolveErr);
    throw resolveErr;
  }
}
