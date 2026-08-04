import fs from 'node:fs';
import path from 'node:path';

import * as prompts from '@voidzero-dev/vite-plus-prompts';
import spawn from 'cross-spawn';

import { rewriteScripts } from '../../../binding/index.js';
import { findUnsafeHookInstallPath, SUPPORTED_GIT_HOOK_NAMES } from '../../config/hooks.ts';
import type { PackageManager, WorkspacePackage } from '../../types/index.ts';
import { editJsonFile, isJsonFile, readJsonFile } from '../../utils/json.ts';
import {
  hasStagedConfigInViteConfig,
  mergeStagedConfigToViteConfig,
  readRulesYaml,
  removeLintStagedFromPackageJson,
} from '../migrator.ts';
import { type MigrationReport } from '../report.ts';
import {
  LINT_STAGED_ALL_CONFIG_FILES,
  LINT_STAGED_OTHER_CONFIG_FILES,
  warnMigration,
} from './shared.ts';

const OTHER_HOOK_TOOLS = ['simple-git-hooks', 'lefthook', 'yorkie'] as const;
const SUPPORTED_GIT_HOOK_NAME_SET = new Set<string>(SUPPORTED_GIT_HOOK_NAMES);

function removeReplacedStagedPackage(packageJsonPath: string): void {
  editJsonFile<{
    devDependencies?: Record<string, string>;
    dependencies?: Record<string, string>;
  }>(packageJsonPath, (pkg) => {
    if (pkg.devDependencies?.['lint-staged']) {
      delete pkg.devDependencies['lint-staged'];
    }
    if (pkg.dependencies?.['lint-staged']) {
      delete pkg.dependencies['lint-staged'];
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
  return pkg['lint-staged'] !== undefined;
}

function hasHuskySetup(
  projectPath: string,
  pkg: {
    scripts?: Record<string, string>;
    devDependencies?: Record<string, string>;
    dependencies?: Record<string, string>;
    optionalDependencies?: Record<string, string>;
    peerDependencies?: Record<string, string>;
    husky?: unknown;
  },
): boolean {
  return (
    pkg.devDependencies?.husky !== undefined ||
    pkg.dependencies?.husky !== undefined ||
    pkg.optionalDependencies?.husky !== undefined ||
    pkg.peerDependencies?.husky !== undefined ||
    pkg.husky !== undefined ||
    (pkg.scripts?.prepare ? /\bhusky\b/.test(pkg.scripts.prepare) : false) ||
    fs.existsSync(path.join(projectPath, '.husky'))
  );
}

export function hasExistingViteHooksPolicy(projectPath: string): boolean {
  const hooksDir = path.join(projectPath, '.vite-hooks');
  const stats = fs.lstatSync(hooksDir, { throwIfNoEntry: false });
  if (!stats?.isDirectory()) {
    return false;
  }
  return fs
    .readdirSync(hooksDir, { withFileTypes: true })
    .some((entry) => entry.isFile() && SUPPORTED_GIT_HOOK_NAME_SET.has(entry.name));
}

function findNonRegularViteHookPath(projectPath: string): string | null {
  for (const hookName of SUPPORTED_GIT_HOOK_NAMES) {
    const hookPath = path.join(projectPath, '.vite-hooks', hookName);
    const stats = fs.lstatSync(hookPath, { throwIfNoEntry: false });
    if (stats && !stats.isFile()) {
      return path.join('.vite-hooks', hookName);
    }
  }
  return null;
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

function findWorkspacePackageHookPolicy(
  workspaceRoot: string,
  packages: readonly WorkspacePackage[],
): string | null {
  for (const pkg of packages) {
    const packagePath = path.join(workspaceRoot, pkg.path);
    if (hasExistingViteHooksPolicy(packagePath)) {
      return `Detected project-owned Vite+ hooks in workspace package "${pkg.path}" — leaving the existing hook setup unchanged.`;
    }

    const packageJsonPath = path.join(packagePath, 'package.json');
    const pkgContent = fs.existsSync(packageJsonPath) ? readJsonFile(packageJsonPath) : {};
    if (hasHuskySetup(packagePath, pkgContent)) {
      return `Detected Husky in workspace package "${pkg.path}" — leaving its hooks, configuration, and dependencies unchanged.`;
    }
    const deps = pkgContent.devDependencies as Record<string, string> | undefined;
    const prodDeps = pkgContent.dependencies as Record<string, string> | undefined;
    for (const tool of OTHER_HOOK_TOOLS) {
      if (deps?.[tool] || prodDeps?.[tool] || pkgContent[tool]) {
        return `Detected ${tool} in workspace package "${pkg.path}" — leaving the existing hook setup unchanged.`;
      }
    }

    // A nested repository can have a package-local hooksPath even when it has
    // no hook-tool dependency or conventional hook directory to detect.
    const gitRoot = findGitRoot(packagePath);
    if (gitRoot && path.resolve(packagePath) === path.resolve(gitRoot)) {
      const configuredHooksPath = getConfiguredHooksPath(packagePath);
      const normalizedHooksPath = normalizeGitHooksPath(configuredHooksPath);
      if (configuredHooksPath && normalizedHooksPath !== '.vite-hooks/_') {
        return `core.hooksPath is already set to "${configuredHooksPath}" in workspace package "${pkg.path}" — leaving the existing hook setup unchanged.`;
      }
    }
  }
  return null;
}

function getConfiguredHooksPath(projectPath: string): string {
  const result = spawn.sync('git', ['config', '--get', 'core.hooksPath'], {
    cwd: projectPath,
    stdio: 'pipe',
  });
  return result.status === 0 ? (result.stdout?.toString().trim() ?? '') : '';
}

function normalizeGitHooksPath(hooksPath: string): string {
  return path.posix.normalize(hooksPath.replaceAll('\\', '/')).replace(/\/$/, '');
}

/**
 * High-level helper that sets up Vite+ hooks when no other hook tool owns the
 * project. Husky setups are deliberately preserved for a dedicated migration.
 */
export function installGitHooks(
  projectPath: string,
  silent = false,
  report?: MigrationReport,
  packageManager?: PackageManager,
  packages: readonly WorkspacePackage[] = [],
): boolean {
  return setupGitHooks(projectPath, silent, report, packageManager, packages);
}

/**
 * Pre-flight check: verify that git hooks can be set up for this project.
 * Returns `null` if hooks setup can proceed, or a warning reason string
 * explaining why hooks setup should be skipped.
 *
 * These checks are deterministic and read-only — they do not modify
 * the project in any way, making them safe to call before migration.
 *
 * The package manager argument remains for API compatibility with callers.
 */
export function preflightGitHooksSetup(
  projectPath: string,
  _packageManager?: PackageManager,
  packages: readonly WorkspacePackage[] = [],
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
  const configuredHooksPath = gitRoot ? getConfiguredHooksPath(projectPath) : '';
  const normalizedHooksPath = normalizeGitHooksPath(configuredHooksPath);
  if (
    hasHuskySetup(projectPath, pkgContent) ||
    normalizedHooksPath === '.husky' ||
    normalizedHooksPath.startsWith('.husky/')
  ) {
    return 'Detected Husky — leaving its hooks, configuration, and dependencies unchanged. Migrate Husky manually before enabling Vite+ hooks.';
  }
  for (const tool of OTHER_HOOK_TOOLS) {
    if (deps?.[tool] || prodDeps?.[tool] || pkgContent[tool]) {
      return `Detected ${tool} — skipping git hooks setup. Please configure git hooks manually, see https://viteplus.dev/guide/migrate#git-hook-tools`;
    }
  }
  const workspacePackageReason = findWorkspacePackageHookPolicy(projectPath, packages);
  if (workspacePackageReason) {
    return workspacePackageReason;
  }
  const disabledHooksEnvironment = ['HUSKY', 'VP_GIT_HOOKS', 'VITE_GIT_HOOKS'].find(
    (name) => process.env[name] === '0',
  );
  if (disabledHooksEnvironment) {
    return `Git hooks are disabled through ${disabledHooksEnvironment}=0 — skipping git hooks setup.`;
  }
  if (configuredHooksPath && normalizedHooksPath !== '.vite-hooks/_') {
    return `core.hooksPath is already set to "${configuredHooksPath}" — leaving the existing hook setup unchanged.`;
  }
  const unsafeInstallPath = findUnsafeHookInstallPath(projectPath, '.vite-hooks');
  if (unsafeInstallPath) {
    return `Git hook dispatcher path "${unsafeInstallPath.relativePath}" is unsafe — leaving the existing hook setup unchanged.`;
  }
  const nonRegularHookPath = findNonRegularViteHookPath(projectPath);
  if (nonRegularHookPath) {
    return `Git hook path "${nonRegularHookPath}" is not a regular file — leaving the existing hook setup unchanged.`;
  }
  if (hasUnsupportedLintStagedConfig(projectPath)) {
    return 'Unsupported lint-staged config format — skipping git hooks setup. Please configure git hooks manually.';
  }
  return null;
}

/**
 * Decide whether config rewriting must leave lint-staged configuration in
 * place. Call this after scaffolding but before rewriting the project so an
 * existing hook owner never observes a partially migrated configuration.
 */
export function shouldSkipStagedMigrationForHooks(
  projectPath: string,
  shouldSetupHooks: boolean,
  packageManager?: PackageManager,
  packages: readonly WorkspacePackage[] = [],
): boolean {
  return (
    !shouldSetupHooks ||
    hasExistingViteHooksPolicy(projectPath) ||
    preflightGitHooksSetup(projectPath, packageManager, packages) !== null
  );
}

/**
 * Set up git hooks with husky + lint-staged via vp commands.
 * Skips if another hook tool is detected (warns user).
 * Returns true if hooks were successfully set up, false if skipped.
 */
export function setupGitHooks(
  projectPath: string,
  silent = false,
  report?: MigrationReport,
  packageManager?: PackageManager,
  packages: readonly WorkspacePackage[] = [],
): boolean {
  const reason = preflightGitHooksSetup(projectPath, packageManager, packages);
  if (reason) {
    warnMigration(reason, report);
    return false;
  }

  const packageJsonPath = path.join(projectPath, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return false;
  }

  const gitRoot = findGitRoot(projectPath);

  const hooksDir = '.vite-hooks';
  const hasExistingHookPolicy = hasExistingViteHooksPolicy(projectPath);

  editJsonFile<{
    scripts?: Record<string, string>;
    devDependencies?: Record<string, string>;
    dependencies?: Record<string, string>;
  }>(packageJsonPath, (pkg) => {
    // Ensure the generated dispatcher is refreshed after dependency installs.
    if (!pkg.scripts) {
      pkg.scripts = {};
    }
    if (!pkg.scripts.prepare) {
      pkg.scripts.prepare = 'vp config';
    } else if (!pkg.scripts.prepare.includes('vp config')) {
      pkg.scripts.prepare = `vp config && ${pkg.scripts.prepare}`;
    }

    return pkg;
  });

  // Add staged config to vite.config.ts if not present
  let stagedMerged = hasStagedConfigInViteConfig(projectPath);
  const hasStandaloneConfig = hasStandaloneLintStagedConfig(projectPath);
  if (!hasExistingHookPolicy && !stagedMerged && !hasStandaloneConfig) {
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
  if (!hasExistingHookPolicy && stagedMerged) {
    removeLintStagedFromPackageJson(packageJsonPath);
  }

  // Only create pre-commit hook if staged config was merged into vite.config.ts.
  // Standalone lint-staged config files are NOT sufficient — `vp staged` only
  // reads from vite.config.ts, so a hook without merged config would fail.
  if (!hasExistingHookPolicy && stagedMerged) {
    createPreCommitHook(projectPath, hooksDir);
  }

  // vp config requires a git workspace — skip if no .git found
  if (!gitRoot) {
    if (!hasExistingHookPolicy && stagedMerged) {
      removeReplacedStagedPackage(packageJsonPath);
    }
    return true;
  }

  const vpBin = process.env.VP_CLI_BIN ?? 'vp';

  // Install git hooks via vp config (--no-agent to skip agent setup, handled by migration)
  const configResult = spawn.sync(vpBin, ['config', '--no-agent'], {
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
    if (!hasExistingHookPolicy && stagedMerged) {
      removeReplacedStagedPackage(packageJsonPath);
    }
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

const DEFAULT_STAGED_CONFIG: Record<string, string> = { '*': 'vp check --fix' };

export function createPreCommitHook(projectPath: string, dir = '.vite-hooks'): void {
  const huskyDir = path.join(projectPath, dir);
  fs.mkdirSync(huskyDir, { recursive: true });
  const hookPath = path.join(huskyDir, 'pre-commit');
  if (fs.existsSync(hookPath)) {
    return;
  }
  fs.writeFileSync(hookPath, 'vp staged\n');
  fs.chmodSync(hookPath, 0o755);
}
