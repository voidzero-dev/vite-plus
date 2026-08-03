import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import * as prompts from '@voidzero-dev/vite-plus-prompts';
import spawn from 'cross-spawn';
import semver from 'semver';

import { rewriteScripts } from '../../../binding/index.js';
import {
  findUnsafeHookInstallPath,
  normalizeHooksPath,
  SUPPORTED_GIT_HOOK_NAMES,
} from '../../config/hooks.ts';
import { PackageManager } from '../../types/index.ts';
import { editJsonFile, isJsonFile, readJsonFile } from '../../utils/json.ts';
import { detectPackageMetadata } from '../../utils/package.ts';
import { detectConfigs } from '../detector.ts';
import {
  createCatalogDependencyResolver,
  hasStagedConfigInViteConfig,
  mergeStagedConfigToViteConfig,
  readPrepareRulesYaml,
  readRulesYaml,
  removeLintStagedFromPackageJson,
  rewriteLintStagedConfigFile,
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

function removeReplacedHookPackages(
  packageJsonPath: string,
  preserveLintStaged = false,
  preserveHusky = false,
): void {
  editJsonFile<{
    devDependencies?: Record<string, string>;
    dependencies?: Record<string, string>;
  }>(packageJsonPath, (pkg) => {
    for (const name of REPLACED_HOOK_PACKAGES) {
      if ((name === 'lint-staged' && preserveLintStaged) || (name === 'husky' && preserveHusky)) {
        continue;
      }
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
  return (
    (pkg.scripts?.prepare ? hasHuskyCommand(pkg.scripts.prepare) : false) ||
    pkg['lint-staged'] !== undefined
  );
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

const STANDARD_HUSKY_PREPARE_RULE = String.raw`---
id: replace-standard-husky
language: bash
rule:
  kind: command
  regex: '^husky(?:[ \t]+(?:init|install(?:[ \t]+(?:\./)?\.husky/?)?|(?:\./)?\.husky/?))?$'
fix: vp config
`;

const VP_CONFIG_DETECTION_RULE = String.raw`---
id: detect-vp-config
language: bash
rule:
  kind: command_name
  regex: '^vp$'
  inside:
    kind: command
    regex: '^vp[ \t]+config(?:$|[ \t])'
fix: __vite_plus_detect_vp_config__
`;

type HuskyPrepareAnalysis =
  | { kind: 'absent' }
  | { kind: 'standard'; rewritten: string }
  | { kind: 'unsupported' };

function hasHuskyCommand(script: string): boolean {
  return Boolean(rewriteScripts(JSON.stringify({ prepare: script }), readPrepareRulesYaml()));
}

function analyzeHuskyPrepareScript(script: string): HuskyPrepareAnalysis {
  if (!hasHuskyCommand(script)) {
    return { kind: 'absent' };
  }
  const updated = rewriteScripts(JSON.stringify({ prepare: script }), STANDARD_HUSKY_PREPARE_RULE);
  if (!updated) {
    return { kind: 'unsupported' };
  }
  const rewritten = (JSON.parse(updated) as { prepare: string }).prepare;
  return hasHuskyCommand(rewritten) ? { kind: 'unsupported' } : { kind: 'standard', rewritten };
}

function hasVpConfigCommand(script: string): boolean {
  return Boolean(rewriteScripts(JSON.stringify({ prepare: script }), VP_CONFIG_DETECTION_RULE));
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
  return setupGitHooks(projectPath, oldHooksDir, silent, report, packageManager);
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
  return analyzeHuskyPrepareScript(pkg.scripts.prepare).kind === 'standard' ? '.husky' : undefined;
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
  oldHooksDir = getOldHooksDir(projectPath),
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
  const prepare = (pkgContent.scripts as Record<string, string> | undefined)?.prepare;
  if (prepare && analyzeHuskyPrepareScript(prepare).kind === 'unsupported') {
    return 'Nonstandard Husky command detected in scripts.prepare — skipping git hooks setup. Vite+ only migrates conventional .husky setups; configure hooks manually.';
  }
  if (oldHooksDir != null && oldHooksDir !== '.husky') {
    return `Custom Husky hook directory "${oldHooksDir}" detected — skipping git hooks setup. Vite+ only migrates the conventional .husky directory.`;
  }
  const disabledHooksEnvironment = getDisabledGitHooksEnvironment();
  if (disabledHooksEnvironment) {
    return `Git hooks are disabled through ${disabledHooksEnvironment} — skipping git hooks setup.`;
  }
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
  const hooksDir = '.vite-hooks';
  const projectHooksDirs = [oldHooksDir, hooksDir].filter((dir): dir is string => dir != null);
  const unsafeHooksDirectory = findUnsafeHooksDirectoryComponent(projectPath, projectHooksDirs);
  if (unsafeHooksDirectory?.kind === 'symbolic') {
    return `Symbolic Git hook path "${unsafeHooksDirectory.relativePath}" cannot be migrated safely — skipping git hooks setup. Replace it with a project-owned file and re-run migration.`;
  }
  if (unsafeHooksDirectory) {
    return `Git hook path "${unsafeHooksDirectory.relativePath}" is not a directory — skipping git hooks setup. Replace it with a directory and re-run migration.`;
  }
  const unsafeInstallPath = findUnsafeHookInstallPath(projectPath, hooksDir);
  if (unsafeInstallPath?.kind === 'symbolic') {
    return `Symbolic Git hook path "${unsafeInstallPath.relativePath}" cannot be migrated safely — skipping git hooks setup. Replace it with a project-owned file and re-run migration.`;
  }
  if (unsafeInstallPath?.kind === 'linked') {
    return `Multiply linked Git hook path "${unsafeInstallPath.relativePath}" cannot be migrated safely — skipping git hooks setup. Replace it with a project-owned file and re-run migration.`;
  }
  if (unsafeInstallPath) {
    const expectedType = unsafeInstallPath.kind === 'not-directory' ? 'directory' : 'file';
    return `Git hook path "${unsafeInstallPath.relativePath}" is not a ${expectedType} — skipping git hooks setup. Replace it with a ${expectedType} and re-run migration.`;
  }
  const unsafeProjectHook = findUnsafeProjectHook(projectPath, projectHooksDirs);
  if (unsafeProjectHook?.kind === 'symbolic') {
    return `Symbolic Git hook path "${unsafeProjectHook.relativePath}" cannot be migrated safely — skipping git hooks setup. Replace it with a project-owned file and re-run migration.`;
  }
  if (unsafeProjectHook?.kind === 'linked') {
    return `Multiply linked Git hook path "${unsafeProjectHook.relativePath}" cannot be migrated safely — skipping git hooks setup. Replace it with a project-owned file and re-run migration.`;
  }
  if (unsafeProjectHook?.kind === 'not-file') {
    return `Git hook path "${unsafeProjectHook.relativePath}" is not a file — skipping git hooks setup. Replace it with a project-owned file and re-run migration.`;
  }
  if (unsafeProjectHook) {
    return `Git hook path "${unsafeProjectHook.relativePath}" is not a regular file or directory — skipping git hooks setup. Replace it with a project-owned entry and re-run migration.`;
  }
  const conflictingHook = findHookMigrationConflict(projectPath, oldHooksDir);
  if (conflictingHook) {
    return `Both .husky/${conflictingHook} and .vite-hooks/${conflictingHook} exist — skipping git hooks setup. Resolve the duplicate hooks and re-run migration.`;
  }
  if (gitRoot) {
    const existingHooksPath = getExistingHooksPath(projectPath);
    const localHooksPath = getLocalHooksPath(projectPath);
    const normalizedExistingHooksPath = normalizeHooksPath(existingHooksPath);
    if (
      existingHooksPath &&
      normalizedExistingHooksPath !== normalizeHooksPath(localHooksPath) &&
      normalizedExistingHooksPath !== normalizeHooksPath(`${hooksDir}/_`)
    ) {
      return `core.hooksPath is already set to "${existingHooksPath}" outside the local repository config, skipping git hooks setup.`;
    }
    if (!canReplaceHooksPath(existingHooksPath, hooksDir, oldHooksDir)) {
      return `core.hooksPath is already set to "${existingHooksPath}", skipping git hooks setup.`;
    }
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
  const reason = preflightGitHooksSetup(projectPath, packageManager, oldHooksDir);
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
  const projectHooksDirs = [oldHooksDir, hooksDir].filter((dir): dir is string => dir != null);
  let transaction: ReturnType<typeof captureGitHooksSetupRollback>;
  try {
    transaction = captureGitHooksSetupRollback(projectPath, projectHooksDirs, report);
  } catch {
    warnMigration('Failed to snapshot the existing Git hook state; no changes were made', report);
    return false;
  }
  const previousHooksPath = gitRoot ? getExistingHooksPath(projectPath) : '';
  const previousLocalHooksPath = gitRoot ? getLocalHooksPath(projectPath) : '';

  try {
    const hasExistingHookPolicy = projectHooksDirs.some((dir) =>
      hasProjectHookScripts(projectPath, dir),
    );
    const hasStagedHookInvocation = projectHooksDirs.some((dir) =>
      hasStagedCommandInProjectHooks(projectPath, dir),
    );

    // Custom hooks keep control of their policy.
    let stagedMerged = hasStagedConfigInViteConfig(projectPath);
    const hasStandaloneConfig = hasStandaloneLintStagedConfig(projectPath);
    let migratedStandaloneConfigPaths: string[] = [];
    if (!stagedMerged && hasStandaloneConfig) {
      migratedStandaloneConfigPaths = rewriteLintStagedConfigFile(projectPath, report, {
        preserveOriginal: true,
      });
      stagedMerged = hasStagedConfigInViteConfig(projectPath);
      if (!stagedMerged) {
        transaction.rollback();
        return false;
      }
    }
    if (!stagedMerged && !hasStandaloneConfig) {
      const pkgData = readJsonFile(packageJsonPath) as {
        'lint-staged'?: Record<string, string | string[]>;
      };
      const stagedConfig =
        pkgData?.['lint-staged'] ??
        (hasStagedHookInvocation || !hasExistingHookPolicy ? DEFAULT_STAGED_CONFIG : undefined);
      if (stagedConfig) {
        const updated = rewriteScripts(JSON.stringify(stagedConfig), readRulesYaml());
        const finalConfig: Record<string, string | string[]> = updated
          ? JSON.parse(updated)
          : stagedConfig;
        stagedMerged = mergeStagedConfigToViteConfig(projectPath, finalConfig, silent, report);
        if (!stagedMerged) {
          transaction.rollback();
          return false;
        }
      }
    }

    editJsonFile<{
      scripts?: Record<string, string>;
      devDependencies?: Record<string, string>;
      dependencies?: Record<string, string>;
    }>(packageJsonPath, (pkg) => {
      // Husky prepare scripts are rewritten after setup succeeds.
      if (!pkg.scripts) {
        pkg.scripts = {};
      }
      if (!pkg.scripts.prepare) {
        pkg.scripts.prepare = 'vp config';
      } else if (!hasVpConfigCommand(pkg.scripts.prepare) && oldHooksDir == null) {
        pkg.scripts.prepare = `vp config && ${pkg.scripts.prepare}`;
      }

      return pkg;
    });

    // vp config requires a git workspace — skip if no .git found
    if (!gitRoot) {
      if (!rewriteDetectedHuskyPrepareScript(projectPath, oldHooksDir)) {
        transaction.rollback();
        warnMigration('Failed to rewrite the Husky prepare script', report);
        return false;
      }
      const preserveLintStaged =
        migrateProjectHooks(
          projectPath,
          oldHooksDir,
          hooksDir,
          stagedMerged,
          hasExistingHookPolicy,
        ) || hasLintStagedReferenceInPackageScripts(packageJsonPath);
      const preserveHusky =
        hasHuskyCommandInProjectHooks(projectPath, hooksDir) ||
        hasHuskyCommandInPackageScripts(packageJsonPath);
      finalizeStagedConfigMigration(
        packageJsonPath,
        migratedStandaloneConfigPaths,
        stagedMerged && !preserveLintStaged,
      );
      removeReplacedHookPackages(packageJsonPath, preserveLintStaged, preserveHusky);
      transaction.discard();
      return true;
    }

    if (oldHooksDir) {
      const normalizedPreviousHooksPath = normalizeHooksPath(previousHooksPath);
      if (
        normalizedPreviousHooksPath === normalizeHooksPath(`${oldHooksDir}/_`) ||
        normalizedPreviousHooksPath === normalizeHooksPath(oldHooksDir)
      ) {
        spawn.sync('git', ['config', '--local', '--unset', 'core.hooksPath'], {
          cwd: projectPath,
          stdio: 'pipe',
        });
      }
    }

    const vpBin = process.env.VP_CLI_BIN ?? 'vp';

    // Install git hooks via vp config (--no-agent to skip agent setup, handled by migration)
    const configArgs = ['config', '--no-agent'];
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
        transaction.rollback();
        restoreLocalHooksPath(projectPath, previousLocalHooksPath);
        warnMigration(`Git hooks not configured — ${stdout}`, report);
        return false;
      }
      if (!rewriteDetectedHuskyPrepareScript(projectPath, oldHooksDir)) {
        transaction.rollback();
        restoreLocalHooksPath(projectPath, previousLocalHooksPath);
        warnMigration('Failed to rewrite the Husky prepare script', report);
        return false;
      }
      const preserveLintStaged =
        migrateProjectHooks(
          projectPath,
          oldHooksDir,
          hooksDir,
          stagedMerged,
          hasExistingHookPolicy,
        ) || hasLintStagedReferenceInPackageScripts(packageJsonPath);
      const preserveHusky =
        hasHuskyCommandInProjectHooks(projectPath, hooksDir) ||
        hasHuskyCommandInPackageScripts(packageJsonPath);
      finalizeStagedConfigMigration(
        packageJsonPath,
        migratedStandaloneConfigPaths,
        stagedMerged && !preserveLintStaged,
      );
      removeReplacedHookPackages(packageJsonPath, preserveLintStaged, preserveHusky);
      transaction.discard();
      if (report) {
        report.gitHooksConfigured = true;
      }
      if (!silent) {
        prompts.log.success('✔ Git hooks configured');
      }
      return true;
    }
    transaction.rollback();
    restoreLocalHooksPath(projectPath, previousLocalHooksPath);
    warnMigration('Failed to install git hooks', report);
    return false;
  } catch {
    transaction.rollback();
    if (gitRoot) {
      restoreLocalHooksPath(projectPath, previousLocalHooksPath);
    }
    warnMigration('Failed to migrate git hooks safely; restored the original hook files', report);
    return false;
  }
}

function rewriteDetectedHuskyPrepareScript(
  projectPath: string,
  oldHooksDir: string | undefined,
): boolean {
  if (oldHooksDir == null) {
    return true;
  }
  try {
    return rewritePrepareScript(projectPath) === oldHooksDir;
  } catch {
    return false;
  }
}

function migrateProjectHooks(
  projectPath: string,
  oldHooksDir: string | undefined,
  hooksDir: string,
  stagedMerged: boolean,
  hasExistingHookPolicy: boolean,
): boolean {
  if (oldHooksDir) {
    const oldDir = path.join(projectPath, oldHooksDir);
    if (fs.existsSync(oldDir)) {
      const targetDir = path.join(projectPath, hooksDir);
      fs.mkdirSync(targetDir, { recursive: true });
      for (const entry of getMigratableHookEntries(oldDir)) {
        const src = path.join(oldDir, entry.relativePath);
        const dest = path.join(targetDir, entry.relativePath);
        if (entry.dirent.isDirectory()) {
          const destinationExists = fs.existsSync(dest);
          fs.mkdirSync(dest, { recursive: true });
          if (!destinationExists) {
            fs.chmodSync(dest, fs.statSync(src).mode & 0o777);
          }
          continue;
        }
        fs.copyFileSync(src, dest);
        fs.chmodSync(dest, fs.statSync(src).mode & 0o777);
      }
      fs.rmSync(oldDir, { recursive: true, force: true });
    }
  }

  removeLegacyHuskyBootstrapFromProjectHooks(projectPath, hooksDir);

  if (!stagedMerged) {
    return hasLintStagedReferenceInProjectHooks(projectPath, hooksDir);
  }
  if (hasExistingHookPolicy) {
    migrateStagedCommandsInProjectHooks(projectPath, hooksDir);
  } else {
    createPreCommitHook(projectPath, hooksDir);
  }
  return hasLintStagedReferenceInProjectHooks(projectPath, hooksDir);
}

function findHookMigrationConflict(
  projectPath: string,
  oldHooksDir: string | undefined,
): string | undefined {
  if (oldHooksDir == null) {
    return undefined;
  }
  const oldDir = path.join(projectPath, oldHooksDir);
  if (!fs.existsSync(oldDir)) {
    return undefined;
  }
  return getMigratableHookEntries(oldDir).find((entry) => {
    const destinationStats = lstatIfExists(
      path.join(projectPath, '.vite-hooks', entry.relativePath),
    );
    if (!destinationStats) {
      return false;
    }
    return !(entry.dirent.isDirectory() && destinationStats.isDirectory());
  })?.relativePath;
}

interface UnsafeProjectHook {
  kind: 'symbolic' | 'linked' | 'not-file' | 'unsupported';
  relativePath: string;
}

function findUnsafeProjectHook(
  projectPath: string,
  dirs: Array<string | undefined>,
): UnsafeProjectHook | undefined {
  for (const dir of new Set(dirs.filter((value): value is string => value != null))) {
    const hooksPath = path.join(projectPath, dir);
    let hooksStats: fs.Stats;
    try {
      hooksStats = fs.lstatSync(hooksPath);
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code === 'ENOENT') {
        continue;
      }
      throw error;
    }
    if (hooksStats.isSymbolicLink()) {
      return { kind: 'symbolic', relativePath: dir };
    }
    if (!hooksStats.isDirectory()) {
      continue;
    }
    for (const entry of getMigratableHookEntries(hooksPath)) {
      const relativePath = path.join(dir, entry.relativePath);
      if (entry.dirent.isSymbolicLink()) {
        return { kind: 'symbolic', relativePath };
      }
      if (
        entry.dirent.isFile() &&
        fs.lstatSync(path.join(hooksPath, entry.relativePath)).nlink > 1
      ) {
        return { kind: 'linked', relativePath };
      }
      const isTopLevelHook =
        !entry.relativePath.includes('/') && GIT_HOOK_NAME_SET.has(entry.relativePath);
      if (isTopLevelHook && !entry.dirent.isFile()) {
        return { kind: 'not-file', relativePath };
      }
      if (!entry.dirent.isFile() && !entry.dirent.isDirectory()) {
        return { kind: 'unsupported', relativePath };
      }
    }
  }
  return undefined;
}

interface UnsafeHooksDirectoryComponent {
  kind: 'symbolic' | 'not-directory';
  relativePath: string;
}

function findUnsafeHooksDirectoryComponent(
  projectPath: string,
  dirs: string[],
): UnsafeHooksDirectoryComponent | undefined {
  const projectRoot = path.resolve(projectPath);

  for (const dir of new Set(dirs)) {
    const hooksPath = path.resolve(projectRoot, dir);
    const relativeHooksPath = path.relative(projectRoot, hooksPath);
    let currentPath = projectRoot;

    for (const component of relativeHooksPath.split(path.sep).filter(Boolean)) {
      currentPath = path.join(currentPath, component);
      const stats = lstatIfExists(currentPath);
      if (!stats) {
        break;
      }
      if (stats.isSymbolicLink()) {
        return { kind: 'symbolic', relativePath: path.relative(projectRoot, currentPath) };
      }
      if (!stats.isDirectory()) {
        return { kind: 'not-directory', relativePath: path.relative(projectRoot, currentPath) };
      }
    }
  }
  return undefined;
}

interface MigratableHookEntry {
  dirent: fs.Dirent;
  relativePath: string;
}

function getMigratableHookEntries(hooksPath: string): MigratableHookEntry[] {
  const result: MigratableHookEntry[] = [];

  function visit(directoryPath: string, relativeDirectory: string): void {
    for (const dirent of fs.readdirSync(directoryPath, { withFileTypes: true })) {
      // Husky owns this dispatcher directory. Everything else is project-owned.
      if (!relativeDirectory && dirent.name === '_') {
        continue;
      }
      const relativePath = relativeDirectory ? `${relativeDirectory}/${dirent.name}` : dirent.name;
      result.push({ dirent, relativePath });
      if (dirent.isDirectory()) {
        visit(path.join(directoryPath, dirent.name), relativePath);
      }
    }
  }

  visit(hooksPath, '');
  return result;
}

function lstatIfExists(filePath: string): fs.Stats | undefined {
  try {
    return fs.lstatSync(filePath);
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === 'ENOENT') {
      return undefined;
    }
    throw error;
  }
}

function getDisabledGitHooksEnvironment(): string | undefined {
  const name = ['HUSKY', 'VP_GIT_HOOKS', 'VITE_GIT_HOOKS'].find(
    (name) => process.env[name] === '0',
  );
  return name ? `${name}=0` : undefined;
}

function getExistingHooksPath(projectPath: string): string {
  const result = spawn.sync('git', ['config', '--get', 'core.hooksPath'], {
    cwd: projectPath,
    stdio: 'pipe',
  });
  return result.status === 0 ? (result.stdout?.toString().trim() ?? '') : '';
}

function getLocalHooksPath(projectPath: string): string {
  const result = spawn.sync('git', ['config', '--local', '--get', 'core.hooksPath'], {
    cwd: projectPath,
    stdio: 'pipe',
  });
  return result.status === 0 ? (result.stdout?.toString().trim() ?? '') : '';
}

function restoreLocalHooksPath(projectPath: string, hooksPath: string): void {
  const args = hooksPath
    ? ['config', '--local', 'core.hooksPath', hooksPath]
    : ['config', '--local', '--unset', 'core.hooksPath'];
  spawn.sync('git', args, { cwd: projectPath, stdio: 'pipe' });
}

function canReplaceHooksPath(
  existingHooksPath: string,
  hooksDir: string,
  oldHooksDir: string | undefined,
): boolean {
  const normalizedExistingHooksPath = normalizeHooksPath(existingHooksPath);
  if (!existingHooksPath || normalizedExistingHooksPath === normalizeHooksPath(`${hooksDir}/_`)) {
    return true;
  }
  if (
    normalizedExistingHooksPath === '.husky' ||
    normalizedExistingHooksPath.startsWith(`.husky${path.sep}`)
  ) {
    return true;
  }
  return (
    oldHooksDir != null &&
    (normalizedExistingHooksPath === normalizeHooksPath(oldHooksDir) ||
      normalizedExistingHooksPath === normalizeHooksPath(`${oldHooksDir}/_`))
  );
}

function captureGitHooksSetupRollback(
  projectPath: string,
  hooksDirs: string[],
  report?: MigrationReport,
): { rollback: () => void; discard: () => void } {
  const packageJsonPath = path.join(projectPath, 'package.json');
  const packageJsonContent = fs.readFileSync(packageJsonPath);
  const existingConfig = detectConfigs(projectPath).viteConfig;
  const existingConfigPath = existingConfig ? path.join(projectPath, existingConfig) : undefined;
  const existingConfigContent = existingConfigPath
    ? fs.readFileSync(existingConfigPath, 'utf8')
    : undefined;
  const standaloneConfigSnapshots = LINT_STAGED_ALL_CONFIG_FILES.flatMap((filename) => {
    const configPath = path.join(projectPath, filename);
    const stats = lstatIfExists(configPath);
    return stats?.isFile()
      ? [{ configPath, content: fs.readFileSync(configPath), mode: stats.mode & 0o777 }]
      : [];
  });
  const reportCounts = report
    ? {
        createdViteConfigCount: report.createdViteConfigCount,
        inlinedLintStagedConfigCount: report.inlinedLintStagedConfigCount,
        mergedStagedConfigCount: report.mergedStagedConfigCount,
      }
    : undefined;
  const backupRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'vite-plus-hooks-rollback-'));
  let hookSnapshots: Array<{ backupPath: string; existed: boolean; hooksPath: string }>;
  try {
    hookSnapshots = [...new Set(hooksDirs.map((dir) => path.resolve(projectPath, dir)))].map(
      (hooksPath, index) => {
        const backupPath = path.join(backupRoot, String(index));
        const existed = fs.existsSync(hooksPath);
        if (existed) {
          fs.cpSync(hooksPath, backupPath, { recursive: true, preserveTimestamps: true });
        }
        return { backupPath, existed, hooksPath };
      },
    );
  } catch (error) {
    fs.rmSync(backupRoot, { recursive: true, force: true });
    throw error;
  }
  let finished = false;

  const discard = () => {
    if (!finished) {
      fs.rmSync(backupRoot, { recursive: true, force: true });
      finished = true;
    }
  };
  const rollback = () => {
    if (finished) {
      return;
    }
    fs.writeFileSync(packageJsonPath, packageJsonContent);
    if (existingConfigPath && existingConfigContent != null) {
      fs.writeFileSync(existingConfigPath, existingConfigContent);
    } else {
      const createdConfig = detectConfigs(projectPath).viteConfig;
      if (createdConfig) {
        fs.rmSync(path.join(projectPath, createdConfig), { force: true });
      }
    }
    if (report && reportCounts) {
      Object.assign(report, reportCounts);
    }
    for (const snapshot of standaloneConfigSnapshots) {
      fs.writeFileSync(snapshot.configPath, snapshot.content);
      fs.chmodSync(snapshot.configPath, snapshot.mode);
    }
    for (const snapshot of hookSnapshots) {
      fs.rmSync(snapshot.hooksPath, { recursive: true, force: true });
      if (snapshot.existed) {
        fs.cpSync(snapshot.backupPath, snapshot.hooksPath, {
          recursive: true,
          preserveTimestamps: true,
        });
      }
    }
    discard();
  };
  return { rollback, discard };
}

function finalizeStagedConfigMigration(
  packageJsonPath: string,
  standaloneConfigPaths: string[],
  stagedMerged: boolean,
): void {
  for (const configPath of standaloneConfigPaths) {
    fs.rmSync(configPath, { force: true });
  }
  if (stagedMerged) {
    removeLintStagedFromPackageJson(packageJsonPath);
  }
}

/**
 * Check if a standalone lint-staged config file exists
 */
function hasStandaloneLintStagedConfig(projectPath: string): boolean {
  return LINT_STAGED_ALL_CONFIG_FILES.some((file) => fs.existsSync(path.join(projectPath, file)));
}

function hasProjectHookScripts(projectPath: string, dir: string): boolean {
  return getProjectHookScriptPaths(projectPath, dir).length > 0;
}

const GIT_HOOK_NAME_SET = new Set<string>(SUPPORTED_GIT_HOOK_NAMES);

function getProjectHookScriptPaths(projectPath: string, dir: string): string[] {
  const hooksPath = path.join(projectPath, dir);
  if (!fs.existsSync(hooksPath)) {
    return [];
  }
  return fs
    .readdirSync(hooksPath, { withFileTypes: true })
    .filter((entry) => GIT_HOOK_NAME_SET.has(entry.name) && entry.isFile())
    .map((entry) => path.join(hooksPath, entry.name));
}

function getProjectHookFilePaths(projectPath: string, dir: string): string[] {
  const hooksPath = path.join(projectPath, dir);
  if (!fs.existsSync(hooksPath)) {
    return [];
  }
  return getMigratableHookEntries(hooksPath)
    .filter((entry) => entry.dirent.isFile())
    .map((entry) => path.join(hooksPath, entry.relativePath));
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
  /^((?:[A-Z_][A-Z0-9_]*(?:=\S*)?\s+)*)(pnpm|pnpm exec|npx|yarn|yarn run|npm exec|npm run|bunx|bun run|bun x)(?:\s+(?:--no-install|--yes|--quiet|-y|-q|--))*\s+lint-staged(?=$|[\s;&|()<>])/,
  /^((?:[A-Z_][A-Z0-9_]*(?:=\S*)?\s+)*)lint-staged(?=$|[\s;&|()<>])/,
];
const VP_STAGED_PATTERN = /^(?:[A-Z_][A-Z0-9_]*(?:=\S*)?\s+)*vp staged(?=$|[\s;&|()<>])/;

const DEFAULT_STAGED_CONFIG: Record<string, string> = { '*': 'vp check --fix' };

function hasStagedCommandInProjectHooks(projectPath: string, dir: string): boolean {
  return getProjectHookFilePaths(projectPath, dir).some((hookPath) => {
    const lines = fs.readFileSync(hookPath, 'utf8').split('\n');
    return lines.some((line) => {
      const trimmed = line.trim();
      if (!trimmed || trimmed.startsWith('#')) {
        return false;
      }
      return (
        STALE_LINT_STAGED_PATTERNS.some((pattern) => pattern.test(trimmed)) ||
        VP_STAGED_PATTERN.test(trimmed)
      );
    });
  });
}

function replaceLintStagedCommand(line: string): string | undefined {
  const trimmed = line.trim();
  for (const pattern of STALE_LINT_STAGED_PATTERNS) {
    const match = pattern.exec(trimmed);
    if (!match) {
      continue;
    }
    const envPrefix = match[1]?.trim() ?? '';
    const rest = trimmed.slice(match[0].length).trim();
    const replacement = [envPrefix, 'vp staged', rest].filter(Boolean).join(' ');
    const start = line.indexOf(trimmed);
    return `${line.slice(0, start)}${replacement}${line.slice(start + trimmed.length)}`;
  }
  return undefined;
}

function migrateStagedCommandsInHook(hookPath: string): boolean {
  const existing = fs.readFileSync(hookPath, 'utf8');
  let changed = false;
  const result = existing.split('\n').map((line) => {
    const replacement = replaceLintStagedCommand(line);
    if (replacement == null) {
      return line;
    }
    changed = true;
    return replacement;
  });
  if (changed) {
    fs.writeFileSync(hookPath, result.join('\n'));
  }
  return changed;
}

function migrateStagedCommandsInProjectHooks(projectPath: string, dir: string): void {
  for (const hookPath of getProjectHookFilePaths(projectPath, dir)) {
    migrateStagedCommandsInHook(hookPath);
  }
}

function hasLintStagedReferenceInProjectHooks(projectPath: string, dir: string): boolean {
  return hasToolReferenceInProjectHooks(projectPath, dir, /\blint-staged\b/);
}

function hasLintStagedReferenceInPackageScripts(packageJsonPath: string): boolean {
  return hasToolReferenceInPackageScripts(packageJsonPath, /\blint-staged\b/);
}

function hasHuskyCommandInProjectHooks(projectPath: string, dir: string): boolean {
  return getProjectHookFilePaths(projectPath, dir).some((hookPath) =>
    hasHuskyCommand(fs.readFileSync(hookPath, 'utf8')),
  );
}

function hasHuskyCommandInPackageScripts(packageJsonPath: string): boolean {
  const pkg = readJsonFile(packageJsonPath) as { scripts?: Record<string, string> };
  return Object.values(pkg.scripts ?? {}).some(hasHuskyCommand);
}

function hasToolReferenceInProjectHooks(
  projectPath: string,
  dir: string,
  pattern: RegExp,
): boolean {
  return getProjectHookFilePaths(projectPath, dir).some((hookPath) =>
    pattern.test(fs.readFileSync(hookPath, 'utf8')),
  );
}

function hasToolReferenceInPackageScripts(packageJsonPath: string, pattern: RegExp): boolean {
  const pkg = readJsonFile(packageJsonPath) as { scripts?: Record<string, string> };
  return Object.values(pkg.scripts ?? {}).some((script) => pattern.test(script));
}

const LEGACY_HUSKY_BOOTSTRAP_PATTERN =
  /^\s*(?:\.|source)\s+["']?\$\(dirname(?:\s+--)?\s+["']?\$0["']?\)\/_\/(?:h|husky\.sh)["']?\s*$/;

function removeLegacyHuskyBootstrapFromProjectHooks(projectPath: string, dir: string): void {
  for (const hookPath of getProjectHookScriptPaths(projectPath, dir)) {
    const existing = fs.readFileSync(hookPath, 'utf8');
    const lines = existing.split('\n');
    const result = lines.filter((line) => !LEGACY_HUSKY_BOOTSTRAP_PATTERN.test(line));
    if (result.length !== lines.length) {
      fs.writeFileSync(hookPath, result.join('\n'));
    }
  }
}

export function createPreCommitHook(projectPath: string, dir = '.vite-hooks'): void {
  const huskyDir = path.join(projectPath, dir);
  fs.mkdirSync(huskyDir, { recursive: true });
  const hookPath = path.join(huskyDir, 'pre-commit');
  if (fs.lstatSync(hookPath, { throwIfNoEntry: false })?.isSymbolicLink()) {
    return;
  }
  if (fs.existsSync(hookPath)) {
    const existing = fs.readFileSync(hookPath, 'utf8');
    if (existing.includes('vp staged')) {
      return; // already has vp staged
    }
    if (!migrateStagedCommandsInHook(hookPath)) {
      fs.writeFileSync(hookPath, `${existing.trimEnd()}\nvp staged\n`);
    }
  } else {
    fs.writeFileSync(hookPath, 'vp staged\n');
    fs.chmodSync(hookPath, 0o755);
  }
}

/**
 * Rewrite only `scripts.prepare` in the root package.json using vite-prepare.yml rules.
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

    const analysis = analyzeHuskyPrepareScript(pkg.scripts.prepare);
    if (analysis.kind === 'standard') {
      oldDir = '.husky';
      pkg.scripts.prepare = analysis.rewritten;
    }
    return pkg;
  });

  return oldDir;
}
