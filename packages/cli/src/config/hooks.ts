import { spawnSync } from 'node:child_process';
import { mkdirSync, rmSync, writeFileSync } from 'node:fs';
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

// The shell script that dispatches to user-defined hooks in .vite-hooks/
const HOOK_SCRIPT = `#!/usr/bin/env sh
{ [ "$HUSKY" = "2" ] || [ "$VITE_GIT_HOOKS" = "2" ]; } && set -x
n=$(basename "$0")
s=$(dirname "$(dirname "$0")")/$n

[ ! -f "$s" ] && exit 0

i="\${XDG_CONFIG_HOME:-$HOME/.config}/vite-plus/init.sh"
[ ! -f "$i" ] && i="\${XDG_CONFIG_HOME:-$HOME/.config}/husky/init.sh"
[ -f "$i" ] && . "$i"

{ [ "\${HUSKY-}" = "0" ] || [ "\${VITE_GIT_HOOKS-}" = "0" ]; } && exit 0

d=$(dirname "$(dirname "$(dirname "$0")")")
export PATH="$d/node_modules/.bin:$PATH"
sh -e "$s" "$@"
c=$?

[ $c != 0 ] && echo "Vite+ - $n script failed (code $c)"
[ $c = 127 ] && echo "Vite+ - command not found in PATH=$PATH"
exit $c`;

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
  const gitRoot = topResult.stdout.toString().trim();

  const internal = (x = '') => join(dir, '_', x);
  const rel = relative(gitRoot, process.cwd());
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
  writeFileSync(internal('h'), HOOK_SCRIPT, { mode: 0o755 });
  for (const hook of HOOKS) {
    writeFileSync(internal(hook), `#!/usr/bin/env sh\n. "$(dirname "$0")/h"`, { mode: 0o755 });
  }
  return { message: '', isError: false };
}
