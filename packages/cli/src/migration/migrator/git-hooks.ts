import fs from 'node:fs';
import path from 'node:path';

import * as prompts from '@voidzero-dev/vite-plus-prompts';
import spawn from 'cross-spawn';

import {
  type LiteralCommandToken,
  parseConfigArgs,
  parseLiteralCommands,
  rewriteScripts,
} from '../../../binding/index.js';
import {
  findUnsafeHookInstallPath,
  HOOK_STUB_SCRIPT,
  hookScript,
  SUPPORTED_GIT_HOOK_NAMES,
} from '../../config/hooks.ts';
import type { PackageManager, WorkspacePackage } from '../../types/index.ts';
import { getVpCommand } from '../../utils/command.ts';
import { documentationUrl } from '../../utils/documentation.ts';
import { findGitRoot } from '../../utils/git.ts';
import { editJsonFile, isJsonFile, readJsonFile } from '../../utils/json.ts';
import {
  hasStagedConfigInViteConfig,
  mergeStagedConfigToViteConfig,
  readRulesYaml,
  removeLintStagedFromPackageJson,
} from '../migrator.ts';
import { type MigrationReport } from '../report.ts';
import {
  getCommandStartIndex,
  LINT_STAGED_ALL_CONFIG_FILES,
  LINT_STAGED_OTHER_CONFIG_FILES,
  SHELL_STATE_COMMANDS,
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

/**
 * List the supported hook files present in `.vite-hooks/`, in canonical order.
 */
export function listExistingViteHookNames(projectPath: string): string[] {
  const hooksDir = path.join(projectPath, '.vite-hooks');
  return SUPPORTED_GIT_HOOK_NAMES.filter((hookName) =>
    fs.lstatSync(path.join(hooksDir, hookName), { throwIfNoEntry: false })?.isFile(),
  );
}

/** Full setup requires project hooks, a refresh lifecycle, and a current,
 * executable dispatcher selected by core.hooksPath at the repository root. */
export function isViteHooksSetupComplete(projectPath: string): boolean {
  if (!hasExistingViteHooksPolicy(projectPath)) {
    return false;
  }
  const packageJsonPath = path.join(projectPath, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return false;
  }
  const pkg = readJsonFile(packageJsonPath) as { scripts?: Record<string, string> };
  if (!hasVpConfigLifecycleScript(pkg.scripts)) {
    return false;
  }
  const gitRoot = findGitRoot(projectPath);
  if (!gitRoot || path.resolve(projectPath) !== path.resolve(gitRoot)) {
    return false;
  }
  if (normalizeGitHooksPath(getConfiguredHooksPath(projectPath)) !== '.vite-hooks/_') {
    return false;
  }
  if (findUnsafeHookInstallPath(projectPath, '.vite-hooks')) {
    return false;
  }
  const dispatcherDir = path.join(projectPath, '.vite-hooks', '_');
  for (const name of ['h', ...listExistingViteHookNames(projectPath)]) {
    const filePath = path.join(dispatcherDir, name);
    const stats = fs.lstatSync(filePath, { throwIfNoEntry: false });
    if (!stats?.isFile()) {
      return false;
    }
    // `vp config` chmods the dispatcher files to 0755; git ignores hooks that
    // are not executable. Windows has no POSIX execute bits to check.
    if (process.platform !== 'win32' && (stats.mode & 0o111) === 0) {
      return false;
    }
    const expectedContent = name === 'h' ? hookScript('.vite-hooks') : HOOK_STUB_SCRIPT;
    if (fs.readFileSync(filePath, 'utf8') !== expectedContent) {
      return false;
    }
  }
  return true;
}

function hasVpConfigLifecycleScript(scripts: Record<string, string> | undefined): boolean {
  return [scripts?.prepare, scripts?.postinstall].some((script) =>
    vpConfigHookCommands(script).some((command) => command.hooksEnabled),
  );
}

interface VpConfigHookCommand {
  args: LiteralCommandToken[];
  hooksEnabled: boolean;
}

/** Only recognize literal invocations in the project directory. Text passed
 * to echo, node, or another command must not count as a dispatcher refresh. */
function vpConfigHookCommands(script: string | undefined): VpConfigHookCommand[] {
  if (!script) {
    return [];
  }
  let commands = parseLiteralCommands(script, true);
  // Preserve a leading refresh even when later commands need shell evaluation.
  // installGitHooks prepends this form when it cannot rewrite an existing script.
  const separator = script.indexOf('&&');
  if (!commands && separator !== -1) {
    const first = parseLiteralCommands(script.slice(0, separator), false)?.[0];
    commands = first?.length ? [first] : null;
  }
  if (!commands) {
    return [];
  }
  const result: VpConfigHookCommand[] = [];
  for (const tokens of commands) {
    const values = tokens.map(({ value }) => value);
    if (/^[A-Za-z_]\w*=/.test(values[0] ?? '')) {
      // Environment overrides can disable hooks or redirect Git operations.
      return result;
    }
    if (SHELL_STATE_COMMANDS.includes(values[0])) {
      return result;
    }
    const start = getCommandStartIndex(values);
    if (values[start] !== 'vp' || values[start + 1] !== 'config') {
      continue;
    }
    const args = tokens.slice(start + 2);
    // The native parser prints help; do not invoke it for help-only scripts.
    if (args.some(({ value }) => value === '--help' || value === '-h')) {
      continue;
    }
    const parsed = parseConfigArgs(args.map(({ value }) => value));
    if (
      parsed.status === 'ok' &&
      (parsed.value.hooksDir === undefined ||
        normalizeGitHooksPath(parsed.value.hooksDir) === '.vite-hooks')
    ) {
      result.push({ args, hooksEnabled: parsed.value.hooks !== false });
    }
  }
  return result;
}

/**
 * Drop `--no-hooks` from every `vp config` invocation in a script, keeping
 * other commands and flags intact. Returns null when the script has no
 * `vp config` invocation to rewrite.
 */
function enableVpConfigHooks(script: string): string | null {
  const flags = vpConfigHookCommands(script).flatMap(({ args }) =>
    args.filter(({ value }) => value === '--no-hooks'),
  );
  if (!flags.length) {
    return null;
  }
  for (const flag of flags.toReversed()) {
    script = script.slice(0, flag.start).trimEnd() + script.slice(flag.end);
  }
  return script;
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
      return `Detected ${tool} — skipping git hooks setup. Please configure git hooks manually, see ${documentationUrl('/guide/migrate#git-hook-tools')}`;
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
 * Set up Vite+ hooks when no other hook tool owns the project.
 * Husky setups are deliberately preserved for a dedicated migration.
 * Returns true if hooks were successfully set up, false if skipped.
 */
export function installGitHooks(
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
    if (!hasVpConfigLifecycleScript(pkg.scripts)) {
      const prepare = pkg.scripts.prepare;
      if (prepare) {
        pkg.scripts.prepare = enableVpConfigHooks(prepare) ?? `vp config && ${prepare}`;
      } else {
        pkg.scripts.prepare = 'vp config';
      }
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

  // Install git hooks via vp config (--no-agent to skip agent setup, handled by migration)
  const { command, args } = getVpCommand(['config', '--no-agent']);
  const configResult = spawn.sync(command, args, {
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
