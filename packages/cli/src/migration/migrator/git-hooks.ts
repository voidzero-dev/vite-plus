import fs from 'node:fs';
import path from 'node:path';

import * as prompts from '@voidzero-dev/vite-plus-prompts';
import spawn from 'cross-spawn';
import semver from 'semver';

import { rewriteScripts } from '../../../binding/index.js';
import { PackageManager } from '../../types/index.ts';
import { editJsonFile, isJsonFile, readJsonFile } from '../../utils/json.ts';
import { detectPackageMetadata } from '../../utils/package.ts';
import {
  createCatalogDependencyResolver,
  hasStagedConfigInViteConfig,
  mergeStagedConfigToViteConfig,
  readPrepareRulesYaml,
  readRulesYaml,
  removeLintStagedFromPackageJson,
} from '../migrator.ts';
import { type MigrationReport } from '../report.ts';
import {
  LINT_STAGED_ALL_CONFIG_FILES,
  LINT_STAGED_OTHER_CONFIG_FILES,
  warnMigration,
} from './shared.ts';

/**
 * Check if the project has an unsupported husky version (<9.0.0).
 * Uses `semver.coerce` to handle ranges like `^8.0.0` → `8.0.0`.
 * When the specifier is a catalog reference (e.g. `"catalog:"`), resolves
 * it from the active package manager's catalog first — a `catalog:` spec is
 * only meaningful to the manager that owns the workspace, so we never read a
 * leftover/foreign catalog file. When it is still not coercible (e.g.
 * `"latest"`), falls back to the installed version in node_modules via
 * `detectPackageMetadata`.
 * Returns a reason string if hooks migration should be skipped, or null
 * if husky is absent or compatible.
 */
function checkUnsupportedHuskyVersion(
  projectPath: string,
  deps: Record<string, string> | undefined,
  prodDeps: Record<string, string> | undefined,
  packageManager: PackageManager | undefined,
): string | null {
  const huskyVersion = deps?.husky ?? prodDeps?.husky;
  if (!huskyVersion) {
    return null;
  }
  let coerced = semver.coerce(huskyVersion);
  if (coerced == null && packageManager != null && huskyVersion.startsWith('catalog:')) {
    const resolved = createCatalogDependencyResolver(projectPath, packageManager)?.(
      huskyVersion,
      'husky',
    );
    if (resolved) {
      coerced = semver.coerce(resolved);
    }
  }
  if (coerced == null) {
    const installed = detectPackageMetadata(projectPath, 'husky');
    if (installed) {
      coerced = semver.coerce(installed.version);
    }
    if (coerced == null) {
      return `Could not determine husky version from "${huskyVersion}" — please specify a semver-compatible version (e.g., "^9.0.0") and re-run migration.`;
    }
  }
  if (semver.satisfies(coerced, '<9.0.0')) {
    return 'Detected husky <9.0.0 — please upgrade to husky v9+ first, then re-run migration.';
  }
  return null;
}

const OTHER_HOOK_TOOLS = ['simple-git-hooks', 'lefthook', 'yorkie'] as const;

// Packages replaced by vite-plus built-in commands and should be removed from devDependencies
const REPLACED_HOOK_PACKAGES = ['husky', 'lint-staged'] as const;

function removeReplacedHookPackages(packageJsonPath: string): void {
  editJsonFile<{
    devDependencies?: Record<string, string>;
    dependencies?: Record<string, string>;
  }>(packageJsonPath, (pkg) => {
    for (const name of REPLACED_HOOK_PACKAGES) {
      if (pkg.devDependencies?.[name]) {
        delete pkg.devDependencies[name];
      }
      if (pkg.dependencies?.[name]) {
        delete pkg.dependencies[name];
      }
    }
    return pkg;
  });
}

export function detectLegacyGitHooksMigrationCandidate(projectPath: string): boolean {
  const packageJsonPath = path.join(projectPath, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return false;
  }
  const pkg = readJsonFile(packageJsonPath) as {
    scripts?: Record<string, string>;
    'lint-staged'?: unknown;
  };
  return getOldHooksDir(projectPath) !== undefined || pkg['lint-staged'] !== undefined;
}

/**
 * Walk up from `startPath` looking for `.git` (directory or file — submodules
 * use a `.git` file).  Returns the directory that contains `.git`, or `null`.
 */
function findGitRoot(startPath: string): string | null {
  let dir = startPath;
  while (true) {
    if (fs.existsSync(path.join(dir, '.git'))) {
      return dir;
    }
    const parent = path.dirname(dir);
    if (parent === dir) {
      return null;
    }
    dir = parent;
  }
}

/**
 * Normalize "husky install [dir]" → "husky [dir]" so downstream regex
 * and ast-grep rules can match a single pattern.
 */
function collapseHuskyInstall(script: string): string {
  return script.replace('husky install ', 'husky ').replace('husky install', 'husky');
}

/**
 * High-level helper: detect old hooks dir, set up git hooks, and rewrite
 * the prepare script.  Returns true if hooks were successfully installed.
 */
export function installGitHooks(
  projectPath: string,
  silent = false,
  report?: MigrationReport,
  packageManager?: PackageManager,
): boolean {
  const oldHooksDir = getOldHooksDir(projectPath);
  if (setupGitHooks(projectPath, oldHooksDir, silent, report, packageManager)) {
    rewritePrepareScript(projectPath);
    return true;
  }
  return false;
}

/**
 * Read-only probe: extract the old husky hooks directory from `scripts.prepare`
 * without modifying package.json. Returns undefined when no husky reference is found.
 */
export function getOldHooksDir(rootDir: string): string | undefined {
  const packageJsonPath = path.join(rootDir, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return undefined;
  }
  const pkg = readJsonFile(packageJsonPath) as { scripts?: { prepare?: string } };
  if (!pkg.scripts?.prepare) {
    return undefined;
  }
  const prepare = collapseHuskyInstall(pkg.scripts.prepare);
  const match = prepare.match(/\bhusky(?:\s+([\w./-]+))?/);
  if (!match) {
    return undefined;
  }
  return match[1] ?? '.husky';
}

/**
 * Pre-flight check: verify that git hooks can be set up for this project.
 * Returns `null` if hooks setup can proceed, or a warning reason string
 * explaining why hooks setup should be skipped.
 *
 * These checks are deterministic and read-only — they do not modify
 * the project in any way, making them safe to call before migration.
 *
 * `packageManager` is the project's detected manager; it scopes `catalog:`
 * resolution to that manager's catalog so a foreign catalog file is ignored.
 */
export function preflightGitHooksSetup(
  projectPath: string,
  packageManager?: PackageManager,
): string | null {
  const gitRoot = findGitRoot(projectPath);
  if (gitRoot && path.resolve(projectPath) !== path.resolve(gitRoot)) {
    return 'Subdirectory project detected — skipping git hooks setup. Configure hooks at the repository root.';
  }
  const packageJsonPath = path.join(projectPath, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return null; // silently skip
  }
  const pkgContent = readJsonFile(packageJsonPath);
  const deps = pkgContent.devDependencies as Record<string, string> | undefined;
  const prodDeps = pkgContent.dependencies as Record<string, string> | undefined;
  for (const tool of OTHER_HOOK_TOOLS) {
    if (deps?.[tool] || prodDeps?.[tool] || pkgContent[tool]) {
      return `Detected ${tool} — skipping git hooks setup. Please configure git hooks manually, see https://viteplus.dev/guide/migrate#git-hook-tools`;
    }
  }
  const huskyReason = checkUnsupportedHuskyVersion(projectPath, deps, prodDeps, packageManager);
  if (huskyReason) {
    return huskyReason;
  }
  if (hasUnsupportedLintStagedConfig(projectPath)) {
    return 'Unsupported lint-staged config format — skipping git hooks setup. Please configure git hooks manually.';
  }
  return null;
}

/**
 * Set up git hooks with husky + lint-staged via vp commands.
 * Skips if another hook tool is detected (warns user).
 * Returns true if hooks were successfully set up, false if skipped.
 */
export function setupGitHooks(
  projectPath: string,
  oldHooksDir?: string,
  silent = false,
  report?: MigrationReport,
  packageManager?: PackageManager,
): boolean {
  const reason = preflightGitHooksSetup(projectPath, packageManager);
  if (reason) {
    warnMigration(reason, report);
    return false;
  }

  const packageJsonPath = path.join(projectPath, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return false;
  }

  const gitRoot = findGitRoot(projectPath);

  // Custom husky dirs (e.g. .config/husky) stay unchanged;
  // only the default .husky dir gets migrated to .vite-hooks.
  const isCustomDir = oldHooksDir != null && oldHooksDir !== '.husky';
  const hooksDir = isCustomDir ? oldHooksDir : '.vite-hooks';

  editJsonFile<{
    scripts?: Record<string, string>;
    devDependencies?: Record<string, string>;
    dependencies?: Record<string, string>;
  }>(packageJsonPath, (pkg) => {
    // Ensure vp config is present for projects that didn't have husky.
    // Skip when prepare contains "husky" — rewritePrepareScript (called after
    // setupGitHooks succeeds) will transform husky → vp config.
    if (!pkg.scripts) {
      pkg.scripts = {};
    }
    if (!pkg.scripts.prepare) {
      pkg.scripts.prepare = 'vp config';
    } else if (
      !pkg.scripts.prepare.includes('vp config') &&
      !/\bhusky\b/.test(pkg.scripts.prepare)
    ) {
      pkg.scripts.prepare = `vp config && ${pkg.scripts.prepare}`;
    }

    return pkg;
  });

  // Add staged config to vite.config.ts if not present
  let stagedMerged = hasStagedConfigInViteConfig(projectPath);
  const hasStandaloneConfig = hasStandaloneLintStagedConfig(projectPath);
  if (!stagedMerged && !hasStandaloneConfig) {
    // Use lint-staged config from package.json if available, otherwise use default
    const pkgData = readJsonFile(packageJsonPath) as {
      'lint-staged'?: Record<string, string | string[]>;
    };
    const stagedConfig = pkgData?.['lint-staged'] ?? DEFAULT_STAGED_CONFIG;
    const updated = rewriteScripts(JSON.stringify(stagedConfig), readRulesYaml());
    const finalConfig: Record<string, string | string[]> = updated
      ? JSON.parse(updated)
      : stagedConfig;
    stagedMerged = mergeStagedConfigToViteConfig(projectPath, finalConfig, silent, report);
  }

  // Only remove lint-staged key from package.json after staged config is
  // confirmed in vite.config.ts — prevents losing config on merge failure
  if (stagedMerged) {
    removeLintStagedFromPackageJson(packageJsonPath);
  }

  // Copy default .husky/ hooks to .vite-hooks/ before creating pre-commit hook.
  // Custom dirs (e.g. .config/husky) are kept in-place — no copy needed.
  if (oldHooksDir && !isCustomDir) {
    const oldDir = path.join(projectPath, oldHooksDir);
    if (fs.existsSync(oldDir)) {
      const targetDir = path.join(projectPath, hooksDir);
      fs.mkdirSync(targetDir, { recursive: true });
      for (const entry of fs.readdirSync(oldDir, { withFileTypes: true })) {
        if (entry.isDirectory() || entry.name.startsWith('.')) {
          continue;
        }
        const src = path.join(oldDir, entry.name);
        const dest = path.join(targetDir, entry.name);
        fs.copyFileSync(src, dest);
        fs.chmodSync(dest, 0o755);
      }
      // Remove old .husky/ directory after copying hooks to .vite-hooks/
      fs.rmSync(oldDir, { recursive: true, force: true });
    }
  }

  // Only create pre-commit hook if staged config was merged into vite.config.ts.
  // Standalone lint-staged config files are NOT sufficient — `vp staged` only
  // reads from vite.config.ts, so a hook without merged config would fail.
  if (stagedMerged) {
    createPreCommitHook(projectPath, hooksDir);
  }

  // vp config requires a git workspace — skip if no .git found
  if (!gitRoot) {
    removeReplacedHookPackages(packageJsonPath);
    return true;
  }

  // Clear husky's core.hooksPath so vp config can set the new one.
  // Only clear if it matches the old husky directory — preserve genuinely custom paths.
  if (oldHooksDir) {
    const checkResult = spawn.sync('git', ['config', '--local', 'core.hooksPath'], {
      cwd: projectPath,
      stdio: 'pipe',
    });
    const existingPath = checkResult.status === 0 ? checkResult.stdout?.toString().trim() : '';
    if (existingPath === `${oldHooksDir}/_` || existingPath === oldHooksDir) {
      spawn.sync('git', ['config', '--local', '--unset', 'core.hooksPath'], {
        cwd: projectPath,
        stdio: 'pipe',
      });
    }
  }

  const vpBin = process.env.VP_CLI_BIN ?? 'vp';

  // Install git hooks via vp config (--no-agent to skip agent setup, handled by migration)
  const configArgs = isCustomDir
    ? ['config', '--no-agent', '--hooks-dir', hooksDir]
    : ['config', '--no-agent'];
  const configResult = spawn.sync(vpBin, configArgs, {
    cwd: projectPath,
    stdio: 'pipe',
  });
  if (configResult.status === 0) {
    // vp config outputs skip/info messages to stdout via log().
    // An empty message means hooks were installed successfully;
    // any non-empty output indicates a skip (HUSKY=0, hooksPath
    // already set, .git not found, etc.).
    const stdout = configResult.stdout?.toString().trim() ?? '';
    if (stdout) {
      warnMigration(`Git hooks not configured — ${stdout}`, report);
      return false;
    }
    removeReplacedHookPackages(packageJsonPath);
    if (report) {
      report.gitHooksConfigured = true;
    }
    if (!silent) {
      prompts.log.success('✔ Git hooks configured');
    }
    return true;
  }
  warnMigration('Failed to install git hooks', report);
  return false;
}

/**
 * Check if a standalone lint-staged config file exists
 */
function hasStandaloneLintStagedConfig(projectPath: string): boolean {
  return LINT_STAGED_ALL_CONFIG_FILES.some((file) => fs.existsSync(path.join(projectPath, file)));
}

/**
 * Check if a standalone lint-staged config exists in a format that can't be
 * auto-migrated to "staged" in vite.config.ts (non-JSON files like .yaml,
 * .mjs, .cjs, .js, or a non-JSON .lintstagedrc).
 */
function hasUnsupportedLintStagedConfig(projectPath: string): boolean {
  for (const filename of LINT_STAGED_OTHER_CONFIG_FILES) {
    if (fs.existsSync(path.join(projectPath, filename))) {
      return true;
    }
  }
  const lintstagedrcPath = path.join(projectPath, '.lintstagedrc');
  if (fs.existsSync(lintstagedrcPath) && !isJsonFile(lintstagedrcPath)) {
    return true;
  }
  return false;
}

/**
 * Create pre-commit hook file in the hooks directory.
 */
// Lint-staged invocation patterns — replaced in-place with `vp staged`.
// The optional prefix group captures env var assignments like `NODE_OPTIONS=... `.
// We still detect old lint-staged patterns to migrate existing hooks.
const STALE_LINT_STAGED_PATTERNS = [
  /^((?:[A-Z_][A-Z0-9_]*(?:=\S*)?\s+)*)(pnpm|pnpm exec|npx|yarn|yarn run|npm exec|npm run|bunx|bun run|bun x)\s+lint-staged\b/,
  /^((?:[A-Z_][A-Z0-9_]*(?:=\S*)?\s+)*)lint-staged\b/,
];

const DEFAULT_STAGED_CONFIG: Record<string, string> = { '*': 'vp check --fix' };

/**
 * Ensure the pre-commit hook exists with `vp staged`, and that
 * vite.config.ts contains a `staged` block (using the default config
 * if none is present). Called by `vp config` after hook installation.
 */
export function ensurePreCommitHook(projectPath: string, dir = '.vite-hooks'): void {
  if (!hasStagedConfigInViteConfig(projectPath)) {
    mergeStagedConfigToViteConfig(projectPath, DEFAULT_STAGED_CONFIG, true);
  }
  createPreCommitHook(projectPath, dir);
}

export function createPreCommitHook(projectPath: string, dir = '.vite-hooks'): void {
  const huskyDir = path.join(projectPath, dir);
  fs.mkdirSync(huskyDir, { recursive: true });
  const hookPath = path.join(huskyDir, 'pre-commit');
  if (fs.existsSync(hookPath)) {
    const existing = fs.readFileSync(hookPath, 'utf8');
    if (existing.includes('vp staged')) {
      return; // already has vp staged
    }
    // Replace old lint-staged invocations in-place, preserve everything else
    const lines = existing.split('\n');
    let replaced = false;
    const result: string[] = [];
    for (const line of lines) {
      const trimmed = line.trim();
      if (!replaced) {
        let matched = false;
        for (const pattern of STALE_LINT_STAGED_PATTERNS) {
          const match = pattern.exec(trimmed);
          if (match) {
            // Preserve env var prefix (capture group 1) and flags/chained commands after lint-staged
            const envPrefix = match[1]?.trim() ?? '';
            const rest = trimmed.slice(match[0].length).trim();
            const parts = [envPrefix, 'vp staged', rest].filter(Boolean);
            result.push(parts.join(' '));
            replaced = true;
            matched = true;
            break;
          }
        }
        if (matched) {
          continue;
        }
      }
      result.push(line);
    }
    if (!replaced) {
      // No lint-staged line found — append after existing content
      fs.writeFileSync(hookPath, `${result.join('\n').trimEnd()}\nvp staged\n`);
    } else {
      fs.writeFileSync(hookPath, result.join('\n'));
    }
  } else {
    fs.writeFileSync(hookPath, 'vp staged\n');
    fs.chmodSync(hookPath, 0o755);
  }
}

/**
 * Rewrite only `scripts.prepare` in the root package.json using vite-prepare.yml rules.
 * Collapses "husky install" → "husky" before applying ast-grep so that the
 * replace-husky rule produces "vp config" with any directory argument preserved.
 * Returns the old husky hooks dir (if any) for migration to .vite-hooks.
 * Called only when hooks are being set up (not with --no-hooks).
 */
export function rewritePrepareScript(rootDir: string): string | undefined {
  const packageJsonPath = path.join(rootDir, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return undefined;
  }

  let oldDir: string | undefined;

  editJsonFile<{ scripts?: Record<string, string> }>(packageJsonPath, (pkg) => {
    if (!pkg.scripts?.prepare) {
      return pkg;
    }

    // Collapse "husky install" → "husky" so the ast-grep rule
    // produces "vp config" with any directory argument preserved.
    const prepare = collapseHuskyInstall(pkg.scripts.prepare);

    const prepareJson = JSON.stringify({ prepare });
    const updated = rewriteScripts(prepareJson, readPrepareRulesYaml());
    if (updated) {
      let newPrepare: string = JSON.parse(updated).prepare;
      newPrepare = newPrepare.replace(
        /\bvp config(?:\s+(?!-)([\w./-]+))?/,
        (_match: string, dir: string | undefined) => {
          // Capture the old husky dir for hook migration.
          // Default husky dir is .husky; custom dirs keep --hooks-dir flag.
          oldDir = dir ?? '.husky';
          return dir ? `vp config --hooks-dir ${dir}` : 'vp config';
        },
      );
      pkg.scripts.prepare = newPrepare;
    } else if (prepare !== pkg.scripts.prepare) {
      // Pre-processing changed the script (husky install → husky)
      // but no rule matched — keep the collapsed form
      pkg.scripts.prepare = prepare;
    }
    return pkg;
  });

  return oldDir;
}
