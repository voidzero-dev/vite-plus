import { spawnSync } from 'node:child_process';
import {
  chmodSync,
  existsSync,
  lstatSync,
  mkdirSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { isAbsolute, join, normalize, relative, resolve, sep } from 'node:path';

export const SUPPORTED_GIT_HOOK_NAMES = [
  'pre-commit',
  'pre-merge-commit',
  'prepare-commit-msg',
  'commit-msg',
  'post-commit',
  'applypatch-msg',
  'pre-applypatch',
  'post-applypatch',
  'pre-rebase',
  'post-rewrite',
  'post-checkout',
  'post-merge',
  'pre-push',
  'pre-auto-gc',
];

export const DEFAULT_HOOKS_DIR = '.vite-hooks';

/** Local git config: user chose `vp hooks disable` (survives prepare / vp config). */
const PREFERENCE_DISABLED_KEY = 'vp.hooks.disabled';
/** Local git config: last hooks directory used by setup/enable. */
const PREFERENCE_DIR_KEY = 'vp.hooks.dir';

// Build nested dirname expression: depth 3 → dirname "$(dirname "$(dirname "$0"))"
function nestedDirname(depth: number): string {
  let expr = '"$0"';
  for (let i = 0; i < depth; i++) {
    expr = `"$(dirname ${expr})"`;
  }
  return expr;
}

// The shell script that dispatches to user-defined hooks in <dir>/
// `depth` = number of path segments in `dir` + 2 (for `_` subdir + hook filename)
export function hookScript(dir: string): string {
  // Count segments: ".vite-hooks" → 1, ".config/husky" → 2
  // Git accepts forward slashes on every platform; Windows also accepts its
  // native backslash. On POSIX, a backslash is a literal filename character.
  const separators = process.platform === 'win32' ? /[\\/]/ : /\//;
  const segments = dir.split(separators).filter((s) => s !== '' && s !== '.').length;
  const depth = segments + 2; // +2 for _ subdir and hook filename
  const rootExpr = nestedDirname(depth);
  return `#!/usr/bin/env sh
{ [ "$HUSKY" = "2" ] || [ "$VP_GIT_HOOKS" = "2" ] || [ "$VITE_GIT_HOOKS" = "2" ]; } && set -x
n=$(basename "$0")
s=$(dirname "$(dirname "$0")")/$n

[ ! -f "$s" ] && exit 0

i="\${XDG_CONFIG_HOME:-$HOME/.config}/vite-plus/hooks-init.sh"
[ ! -f "$i" ] && i="\${XDG_CONFIG_HOME:-$HOME/.config}/husky/init.sh"
[ -f "$i" ] && . "$i"

{ [ "\${HUSKY-}" = "0" ] || [ "\${VP_GIT_HOOKS-}" = "0" ] || [ "\${VITE_GIT_HOOKS-}" = "0" ]; } && exit 0

d=${rootExpr}
__vp_shell=/bin/sh
[ -x "$__vp_shell" ] || __vp_shell=$(command -v sh)

if [ -n "\${VP_HOME-}" ]; then
  __vp_bin="$VP_HOME/bin"
elif [ -n "\${HOME-}" ]; then
  __vp_bin="$HOME/.vite-plus/bin"
else
  __vp_bin=""
fi
[ -n "$__vp_bin" ] && [ -d "$__vp_bin" ] && export PATH="$PATH:$__vp_bin"

export PATH="$d/node_modules/.bin:$PATH"
"$__vp_shell" -e "$s" "$@"
c=$?

[ $c != 0 ] && echo "VITE+ - $n script failed (code $c)"
[ $c = 127 ] && echo "VITE+ - command not found in PATH=$PATH"
exit $c`;
}

export interface InstallResult {
  message: string;
  isError: boolean;
}

export interface UnsafeHookInstallPath {
  kind: 'symbolic' | 'linked' | 'not-directory' | 'not-file';
  relativePath: string;
}

export interface HooksStatus {
  hooksDir: string;
  userDisabled: boolean;
  hooksPath: string | null;
  ownsHooksPath: boolean;
  dispatcherInstalled: boolean;
  projectHooks: string[];
  lines: string[];
}

export function normalizeHooksPath(hooksPath: string): string {
  let normalized = normalize(hooksPath);
  while (normalized.endsWith(sep)) {
    normalized = normalized.slice(0, -1);
  }
  return normalized;
}

export function findUnsafeHookInstallPath(root: string, dir: string): UnsafeHookInstallPath | null {
  const projectRoot = resolve(root);
  const internalPath = resolve(projectRoot, dir, '_');
  const relativeInternalPath = relative(projectRoot, internalPath);
  let currentPath = projectRoot;

  for (const component of relativeInternalPath.split(sep).filter(Boolean)) {
    currentPath = join(currentPath, component);
    const stats = lstatSync(currentPath, { throwIfNoEntry: false });
    if (!stats) {
      return null;
    }
    if (stats.isSymbolicLink()) {
      return { kind: 'symbolic', relativePath: relative(projectRoot, currentPath) };
    }
    if (!stats.isDirectory()) {
      return { kind: 'not-directory', relativePath: relative(projectRoot, currentPath) };
    }
  }

  for (const filename of ['husky.sh', '.gitignore', 'h', ...SUPPORTED_GIT_HOOK_NAMES]) {
    const filePath = join(internalPath, filename);
    const stats = lstatSync(filePath, { throwIfNoEntry: false });
    if (!stats) {
      continue;
    }
    if (stats.isSymbolicLink()) {
      return { kind: 'symbolic', relativePath: relative(projectRoot, filePath) };
    }
    if (!stats.isFile()) {
      return { kind: 'not-file', relativePath: relative(projectRoot, filePath) };
    }
    if (stats.nlink > 1) {
      return { kind: 'linked', relativePath: relative(projectRoot, filePath) };
    }
  }
  return null;
}

function describeUnsafeHookInstallPath(unsafePath: UnsafeHookInstallPath): string {
  if (unsafePath.kind === 'symbolic') {
    return `symbolic hook path "${unsafePath.relativePath}" not allowed`;
  }
  if (unsafePath.kind === 'linked') {
    return `multiply linked hook path "${unsafePath.relativePath}" not allowed`;
  }
  if (unsafePath.kind === 'not-directory') {
    return `hook path "${unsafePath.relativePath}" is not a directory`;
  }
  return `hook path "${unsafePath.relativePath}" is not a file`;
}

function gitConfigGet(key: string, options?: { local?: boolean; bool?: boolean }): string | null {
  const args = ['config'];
  if (options?.local) {
    args.push('--local');
  }
  if (options?.bool) {
    args.push('--bool');
  }
  args.push('--get', key);
  const result = spawnSync('git', args);
  if (result.status !== 0) {
    return null;
  }
  return result.stdout?.toString().trim() || null;
}

function gitConfigSet(key: string, value: string): { ok: boolean; error?: string } {
  const result = spawnSync('git', ['config', '--local', key, value]);
  if (result.status == null) {
    return { ok: false, error: 'git command not found' };
  }
  if (result.status !== 0) {
    return {
      ok: false,
      error: result.stderr?.toString().trim() || `failed to set ${key}`,
    };
  }
  return { ok: true };
}

function gitConfigUnset(key: string): { ok: boolean; error?: string } {
  const result = spawnSync('git', ['config', '--local', '--unset', key]);
  if (result.status == null) {
    return { ok: false, error: 'git command not found' };
  }
  // status 5 = key not found
  if (result.status !== 0 && result.status !== 5) {
    return {
      ok: false,
      error: result.stderr?.toString().trim() || `failed to unset ${key}`,
    };
  }
  return { ok: true };
}

/** Whether the user ran `vp hooks disable` in this repo (local git config). */
export function isHooksUserDisabled(): boolean {
  return gitConfigGet(PREFERENCE_DISABLED_KEY, { local: true, bool: true }) === 'true';
}

export function setHooksUserDisabled(disabled: boolean): { ok: boolean; error?: string } {
  if (disabled) {
    return gitConfigSet(PREFERENCE_DISABLED_KEY, 'true');
  }
  return gitConfigUnset(PREFERENCE_DISABLED_KEY);
}

export function getStoredHooksDir(): string | null {
  return gitConfigGet(PREFERENCE_DIR_KEY, { local: true });
}

export function setStoredHooksDir(dir: string): { ok: boolean; error?: string } {
  return gitConfigSet(PREFERENCE_DIR_KEY, dir);
}

/**
 * Resolve the hooks directory: CLI flag > stored preference > default.
 */
export function resolveHooksDir(dir?: string): string {
  if (dir) {
    return dir;
  }
  return getStoredHooksDir() ?? DEFAULT_HOOKS_DIR;
}

function validateHooksDir(dir: string): InstallResult | null {
  if (dir.includes('..')) {
    return { message: '.. not allowed', isError: true };
  }
  if (isAbsolute(dir)) {
    return { message: 'absolute hooks directory not allowed', isError: true };
  }
  if (relative(process.cwd(), resolve(process.cwd(), dir)) === '') {
    return { message: 'hooks directory must be a project subdirectory', isError: true };
  }
  return null;
}

function computeTarget(dir: string): { target: string } | InstallResult {
  const prefixResult = spawnSync('git', ['rev-parse', '--show-prefix']);
  if (prefixResult.status == null) {
    return { message: 'git command not found', isError: true };
  }
  if (prefixResult.status !== 0) {
    return { message: ".git can't be found", isError: false };
  }
  const rel = prefixResult.stdout.toString().trim().replace(/\/$/, '');
  const target = rel ? `${rel}/${dir}/_` : `${dir}/_`;
  return { target };
}

function getEffectiveHooksPath(): string {
  const checkResult = spawnSync('git', ['config', '--get', 'core.hooksPath']);
  return checkResult.status === 0 ? checkResult.stdout?.toString().trim() : '';
}

function getScopedHooksPath(scope: 'local' | 'worktree'): string {
  const result = spawnSync('git', ['config', `--${scope}`, '--get', 'core.hooksPath']);
  return result.status === 0 ? result.stdout?.toString().trim() : '';
}

function unsetScopedHooksPath(scope: 'local' | 'worktree'): InstallResult | null {
  const result = spawnSync('git', ['config', `--${scope}`, '--unset', 'core.hooksPath']);
  if (result.status == null) {
    return { message: 'git command not found', isError: true };
  }
  // status 5 = key not found at that scope
  if (result.status !== 0 && result.status !== 5) {
    return {
      message: result.stderr?.toString().trim() || `failed to unset ${scope} core.hooksPath`,
      isError: true,
    };
  }
  return null;
}

/**
 * Unset core.hooksPath only at scopes that actually point at our dispatcher.
 *
 * Must not touch a foreign value at another scope (e.g. local still `.husky/_`
 * while worktree holds the Vite+ target).
 */
function unsetOwnedHooksPath(target: string): InstallResult | null {
  const normalizedTarget = normalizeHooksPath(target);

  for (const scope of ['local', 'worktree'] as const) {
    const scopedPath = getScopedHooksPath(scope);
    if (!scopedPath || normalizeHooksPath(scopedPath) !== normalizedTarget) {
      continue;
    }
    const unsetError = unsetScopedHooksPath(scope);
    if (unsetError) {
      return unsetError;
    }
  }

  const finalPath = getEffectiveHooksPath();
  if (finalPath && normalizeHooksPath(finalPath) === normalizedTarget) {
    return {
      message: `could not unset core.hooksPath (still "${finalPath}"); remove it with git config --unset core.hooksPath`,
      isError: true,
    };
  }

  return null;
}

export interface InstallOptions {
  /**
   * When true, ignore a user-disabled preference (used by `vp hooks setup` / `enable`).
   * Still honors `VP_GIT_HOOKS=0` / `HUSKY=0`.
   */
  ignoreUserPreference?: boolean;
}

export function install(dir = DEFAULT_HOOKS_DIR, options: InstallOptions = {}): InstallResult {
  // VP_GIT_HOOKS is the canonical name; VITE_GIT_HOOKS is kept for backwards compatibility.
  if (
    process.env.HUSKY === '0' ||
    process.env.VP_GIT_HOOKS === '0' ||
    process.env.VITE_GIT_HOOKS === '0'
  ) {
    return { message: 'skip install (git hooks disabled)', isError: false };
  }
  if (!options.ignoreUserPreference && isHooksUserDisabled()) {
    return {
      message: 'skip install (hooks disabled; run `vp hooks enable` to re-enable)',
      isError: false,
    };
  }
  const dirError = validateHooksDir(dir);
  if (dirError) {
    return dirError;
  }
  const unsafeInstallPath = findUnsafeHookInstallPath(process.cwd(), dir);
  if (unsafeInstallPath) {
    return { message: describeUnsafeHookInstallPath(unsafeInstallPath), isError: false };
  }
  // Use --show-prefix to get the relative path from git root to cwd.
  // This avoids Windows path normalization issues (MSYS paths, 8.3 short names)
  // that make path.relative() unreliable across git and Node.js representations.
  const targetResult = computeTarget(dir);
  if ('message' in targetResult) {
    return targetResult;
  }
  const { target } = targetResult;

  const internal = (x = '') => join(dir, '_', x);
  // Read the effective value so a worktree-scoped setting cannot silently
  // override the local value we are about to write.
  const existingHooksPath = getEffectiveHooksPath();
  if (existingHooksPath && normalizeHooksPath(existingHooksPath) !== normalizeHooksPath(target)) {
    return {
      message: `core.hooksPath is already set to "${existingHooksPath}", skipping`,
      isError: false,
    };
  }

  rmSync(internal('husky.sh'), { force: true });
  mkdirSync(internal(), { recursive: true });
  writeFileSync(internal('.gitignore'), '*');
  writeFileSync(internal('h'), hookScript(dir), { mode: 0o755 });
  chmodSync(internal('h'), 0o755);
  for (const hook of SUPPORTED_GIT_HOOK_NAMES) {
    writeFileSync(internal(hook), `#!/usr/bin/env sh\n. "$(dirname "$0")/h"`, { mode: 0o755 });
    chmodSync(internal(hook), 0o755);
  }
  const { status, stderr } = spawnSync('git', ['config', 'core.hooksPath', target]);
  if (status == null) {
    return { message: 'git command not found', isError: true };
  }
  if (status) {
    return { message: '' + stderr, isError: true };
  }

  // Persist enabled state + directory for later enable/disable/status.
  const clearDisabled = setHooksUserDisabled(false);
  if (!clearDisabled.ok) {
    return {
      message: clearDisabled.error || 'failed to clear hooks disabled preference',
      isError: true,
    };
  }
  const storeDir = setStoredHooksDir(dir);
  if (!storeDir.ok) {
    return { message: storeDir.error || 'failed to store hooks directory', isError: true };
  }

  return { message: '', isError: false };
}

/**
 * Install (or refresh) the Vite+ hook dispatcher and mark hooks as enabled.
 * Clears a previous `vp hooks disable` preference.
 */
export function setup(dir = DEFAULT_HOOKS_DIR): InstallResult {
  const result = install(dir, { ignoreUserPreference: true });
  if (result.isError) {
    return result;
  }
  if (result.message) {
    // Non-error skip messages (env disabled, foreign hooksPath, unsafe path, etc.)
    return result;
  }
  return {
    message: `Git hook dispatcher installed at ${dir}/_`,
    isError: false,
  };
}

/**
 * Re-enable hooks after `disable` (same as setup).
 */
export function enable(dir = DEFAULT_HOOKS_DIR): InstallResult {
  return setup(dir);
}

/**
 * Disable Vite+ hooks in this repo and tear down the dispatcher.
 *
 * - Persists the decision in local git config so `vp config` / prepare do not reinstall
 * - Unsets `core.hooksPath` only when it points at this project's dispatcher
 * - Removes the generated `<dir>/_` directory
 * - Leaves project-owned hooks, staged config, and package.json scripts alone
 */
export function disable(dir = DEFAULT_HOOKS_DIR): InstallResult {
  const dirError = validateHooksDir(dir);
  if (dirError) {
    return dirError;
  }

  const targetResult = computeTarget(dir);
  if ('message' in targetResult) {
    return targetResult;
  }
  const { target } = targetResult;
  const internalDir = join(dir, '_');
  const hasInternalDir = existsSync(internalDir);

  // Refuse unsafe trees before any git config mutation so we never leave a
  // partial teardown (hooksPath cleared but `_/` still present).
  if (hasInternalDir) {
    const unsafeInstallPath = findUnsafeHookInstallPath(process.cwd(), dir);
    if (unsafeInstallPath) {
      return {
        message: describeUnsafeHookInstallPath(unsafeInstallPath),
        isError: false,
      };
    }
  }

  const existingHooksPath = getEffectiveHooksPath();
  const ownsHooksPath =
    !!existingHooksPath && normalizeHooksPath(existingHooksPath) === normalizeHooksPath(target);
  const foreignHooksPath =
    !!existingHooksPath && normalizeHooksPath(existingHooksPath) !== normalizeHooksPath(target);

  const actions: string[] = [];
  const notes: string[] = [];

  // Persist preference *before* teardown so a later unset/rm failure still
  // blocks prepare / `vp config` from reinstalling the dispatcher.
  const pref = setHooksUserDisabled(true);
  if (!pref.ok) {
    return { message: pref.error || 'failed to persist hooks disabled preference', isError: true };
  }
  const storeDir = setStoredHooksDir(dir);
  if (!storeDir.ok) {
    return { message: storeDir.error || 'failed to store hooks directory', isError: true };
  }
  actions.push('recorded disable preference (local git config)');

  if (ownsHooksPath) {
    const unsetError = unsetOwnedHooksPath(target);
    if (unsetError) {
      return {
        message: `${unsetError.message}; disable preference was recorded (local git config). Run \`vp hooks enable\` to clear it, or \`git config --local --unset vp.hooks.disabled\``,
        isError: true,
      };
    }
    actions.push(`unset core.hooksPath (was "${existingHooksPath}")`);
  } else if (foreignHooksPath) {
    notes.push(
      `core.hooksPath is set to "${existingHooksPath}" (not Vite+ dispatcher "${target}"), left unchanged`,
    );
  }

  if (hasInternalDir) {
    rmSync(internalDir, { recursive: true, force: true });
    actions.push(`removed ${internalDir}`);
  }

  const summary = `Git hooks disabled: ${actions.join('; ')}. Project-owned hooks under ${dir}/ and staged config were left unchanged. Run \`vp hooks enable\` to re-enable.`;
  if (notes.length > 0) {
    return { message: `${summary} ${notes.join('; ')}.`, isError: false };
  }
  return { message: summary, isError: false };
}

/**
 * Report whether Vite+ hooks are set up, disabled by preference, and active.
 */
export function status(dir?: string): InstallResult & { status?: HooksStatus } {
  const hooksDir = resolveHooksDir(dir);
  if (dir) {
    const dirError = validateHooksDir(dir);
    if (dirError) {
      return dirError;
    }
  } else {
    const dirError = validateHooksDir(hooksDir);
    if (dirError) {
      return dirError;
    }
  }

  const prefixResult = spawnSync('git', ['rev-parse', '--show-prefix']);
  if (prefixResult.status == null) {
    return { message: 'git command not found', isError: true };
  }
  if (prefixResult.status !== 0) {
    return { message: ".git can't be found", isError: false };
  }

  const rel = prefixResult.stdout.toString().trim().replace(/\/$/, '');
  const target = rel ? `${rel}/${hooksDir}/_` : `${hooksDir}/_`;
  const existingHooksPath = getEffectiveHooksPath();
  const userDisabled = isHooksUserDisabled();
  const dispatcherInstalled = existsSync(join(hooksDir, '_', 'h'));
  const ownsHooksPath =
    !!existingHooksPath && normalizeHooksPath(existingHooksPath) === normalizeHooksPath(target);

  let projectHooks: string[] = [];
  if (existsSync(hooksDir)) {
    try {
      projectHooks = readdirSync(hooksDir, { withFileTypes: true })
        .filter((entry) => entry.isFile() && SUPPORTED_GIT_HOOK_NAMES.includes(entry.name))
        .map((entry) => entry.name)
        .toSorted();
    } catch {
      projectHooks = [];
    }
  }

  // Preference is the stored user decision, not runtime activity.
  // - disabled (local): user ran `vp hooks disable`
  // - enabled: setup/enable ran (or hooks are currently owned/installed)
  // - not set: no disable preference and no evidence of a prior setup
  const preferenceLabel = userDisabled
    ? 'disabled (local)'
    : getStoredHooksDir() || dispatcherInstalled || ownsHooksPath
      ? 'enabled'
      : 'not set';
  const hooksPathLabel = existingHooksPath || '(unset)';
  const ownership = !existingHooksPath
    ? ''
    : ownsHooksPath
      ? ' (Vite+ dispatcher)'
      : ' (not Vite+ dispatcher)';
  const dispatcherLabel = dispatcherInstalled ? 'installed' : 'missing';
  const projectHooksLabel = projectHooks.length > 0 ? projectHooks.join(', ') : '(none)';

  const lines = [
    `Preference:     ${preferenceLabel}`,
    `Hooks dir:      ${hooksDir}`,
    `core.hooksPath: ${hooksPathLabel}${ownership}`,
    `Dispatcher:     ${dispatcherLabel} (${hooksDir}/_)`,
    `Project hooks:  ${projectHooksLabel}`,
  ];

  const hooksStatus: HooksStatus = {
    hooksDir,
    userDisabled,
    hooksPath: existingHooksPath || null,
    ownsHooksPath,
    dispatcherInstalled,
    projectHooks,
    lines,
  };

  return {
    message: lines.join('\n'),
    isError: false,
    status: hooksStatus,
  };
}
