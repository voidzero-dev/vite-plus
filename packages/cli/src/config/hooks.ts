import { spawnSync } from 'node:child_process';
import { chmodSync, lstatSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
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

export function install(dir = '.vite-hooks'): InstallResult {
  // VP_GIT_HOOKS is the canonical name; VITE_GIT_HOOKS is kept for backwards compatibility.
  if (
    process.env.HUSKY === '0' ||
    process.env.VP_GIT_HOOKS === '0' ||
    process.env.VITE_GIT_HOOKS === '0'
  ) {
    return { message: 'skip install (git hooks disabled)', isError: false };
  }
  if (dir.includes('..')) {
    return { message: '.. not allowed', isError: false };
  }
  if (isAbsolute(dir)) {
    return { message: 'absolute hooks directory not allowed', isError: false };
  }
  if (relative(process.cwd(), resolve(process.cwd(), dir)) === '') {
    return { message: 'hooks directory must be a project subdirectory', isError: false };
  }
  const unsafeInstallPath = findUnsafeHookInstallPath(process.cwd(), dir);
  if (unsafeInstallPath) {
    return { message: describeUnsafeHookInstallPath(unsafeInstallPath), isError: false };
  }
  // Use --show-prefix to get the relative path from git root to cwd.
  // This avoids Windows path normalization issues (MSYS paths, 8.3 short names)
  // that make path.relative() unreliable across git and Node.js representations.
  const prefixResult = spawnSync('git', ['rev-parse', '--show-prefix']);
  if (prefixResult.status == null) {
    return { message: 'git command not found', isError: true };
  }
  if (prefixResult.status !== 0) {
    return { message: ".git can't be found", isError: false };
  }

  const internal = (x = '') => join(dir, '_', x);
  const rel = prefixResult.stdout.toString().trim().replace(/\/$/, '');
  const target = rel ? `${rel}/${dir}/_` : `${dir}/_`;
  // Read the effective value so a worktree-scoped setting cannot silently
  // override the local value we are about to write.
  const checkResult = spawnSync('git', ['config', '--get', 'core.hooksPath']);
  const existingHooksPath = checkResult.status === 0 ? checkResult.stdout?.toString().trim() : '';
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
  return { message: '', isError: false };
}
