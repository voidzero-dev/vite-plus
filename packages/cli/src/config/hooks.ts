import { spawnSync } from 'node:child_process';
import { mkdirSync, realpathSync, rmSync, writeFileSync } from 'node:fs';
import { join, relative } from 'node:path';

const HOOKS = [
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
function hookScript(dir: string): string {
  // Count segments: ".vite-hooks" → 1, ".config/husky" → 2
  const segments = dir.split('/').filter(Boolean).length;
  const depth = segments + 2; // +2 for _ subdir and hook filename
  const rootExpr = nestedDirname(depth);
  return `#!/usr/bin/env sh
{ [ "$HUSKY" = "2" ] || [ "$VITE_GIT_HOOKS" = "2" ]; } && set -x
n=$(basename "$0")
s=$(dirname "$(dirname "$0")")/$n

[ ! -f "$s" ] && exit 0

i="\${XDG_CONFIG_HOME:-$HOME/.config}/vite-plus/hooks-init.sh"
[ ! -f "$i" ] && i="\${XDG_CONFIG_HOME:-$HOME/.config}/husky/init.sh"
[ -f "$i" ] && . "$i"

{ [ "\${HUSKY-}" = "0" ] || [ "\${VITE_GIT_HOOKS-}" = "0" ]; } && exit 0

d=${rootExpr}
export PATH="$d/node_modules/.bin:$PATH"
sh -e "$s" "$@"
c=$?

[ $c != 0 ] && echo "VITE+ - $n script failed (code $c)"
[ $c = 127 ] && echo "VITE+ - command not found in PATH=$PATH"
exit $c`;
}

export interface InstallResult {
  message: string;
  isError: boolean;
}

export function install(dir = '.vite-hooks'): InstallResult {
  if (process.env.HUSKY === '0' || process.env.VITE_GIT_HOOKS === '0') {
    return { message: 'skip install (git hooks disabled)', isError: false };
  }
  if (dir.includes('..')) {
    return { message: '.. not allowed', isError: false };
  }
  const topResult = spawnSync('git', ['rev-parse', '--show-toplevel']);
  if (topResult.status == null) {
    return { message: 'git command not found', isError: true };
  }
  if (topResult.status !== 0) {
    return { message: ".git can't be found", isError: false };
  }
  let gitRoot = topResult.stdout.toString().trim();
  if (process.platform === 'win32') {
    // Convert MSYS-style paths (e.g. /d/a/repo) to native Windows paths (e.g. D:/a/repo)
    if (/^\/[a-zA-Z]\//.test(gitRoot)) {
      gitRoot = gitRoot[1].toUpperCase() + ':' + gitRoot.slice(2);
    }
    // Resolve 8.3 short names (e.g. RUNNER~1 vs runner) for consistent path comparison
    gitRoot = realpathSync(gitRoot);
  }

  const internal = (x = '') => join(dir, '_', x);
  const cwd = process.platform === 'win32' ? realpathSync(process.cwd()) : process.cwd();
  const rel = relative(gitRoot, cwd);
  const target = rel ? `${rel}/${dir}/_` : `${dir}/_`;
  const checkResult = spawnSync('git', ['config', '--local', 'core.hooksPath']);
  const existingHooksPath = checkResult.status === 0 ? checkResult.stdout?.toString().trim() : '';
  if (existingHooksPath && existingHooksPath !== target) {
    return {
      message: `core.hooksPath is already set to "${existingHooksPath}", skipping`,
      isError: false,
    };
  }

  const { status, stderr } = spawnSync('git', ['config', 'core.hooksPath', target]);
  if (status == null) {
    return { message: 'git command not found', isError: true };
  }
  if (status) {
    return { message: '' + stderr, isError: true };
  }

  rmSync(internal('husky.sh'), { force: true });
  mkdirSync(internal(), { recursive: true });
  writeFileSync(internal('.gitignore'), '*');
  writeFileSync(internal('h'), hookScript(dir), { mode: 0o755 });
  for (const hook of HOOKS) {
    writeFileSync(internal(hook), `#!/usr/bin/env sh\n. "$(dirname "$0")/h"`, { mode: 0o755 });
  }
  return { message: '', isError: false };
}
