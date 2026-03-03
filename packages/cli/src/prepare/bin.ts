// Inlined husky install logic — cannot use husky's default export directly
// because it references `new URL('husky', import.meta.url)` which breaks
// when bundled by rolldown (the `husky` shell script won't be next to the output).
//
// This is a faithful reimplementation of husky v9's install function.

import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

import mri from 'mri';

import { renderCliDoc } from '../utils/help.js';
import { getVitePlusHeader, log } from '../utils/terminal.js';

const helpMessage = renderCliDoc({
  usage: 'vp prepare',
  summary: 'Set up Git hooks for the project (bundled husky).',
  sections: [
    {
      title: 'Options',
      rows: [{ label: '-h, --help', description: 'Show this help message' }],
    },
    {
      title: 'Environment',
      rows: [{ label: 'HUSKY=0', description: 'Skip hook installation' }],
    },
  ],
});

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

// The shell script that dispatches to user-defined hooks in .husky/
const HOOK_SCRIPT = `#!/usr/bin/env sh
[ "$HUSKY" = "2" ] && set -x
n=$(basename "$0")
s=$(dirname "$(dirname "$0")")/$n

[ ! -f "$s" ] && exit 0

i="\${XDG_CONFIG_HOME:-$HOME/.config}/husky/init.sh"
[ -f "$i" ] && . "$i"

[ "\${HUSKY-}" = "0" ] && exit 0

export PATH="node_modules/.bin:$PATH"
sh -e "$s" "$@"
c=$?

[ $c != 0 ] && echo "husky - $n script failed (code $c)"
[ $c = 127 ] && echo "husky - command not found in PATH=$PATH"
exit $c`;

function install(dir = '.husky'): string {
  if (process.env.HUSKY === '0') {
    return 'HUSKY=0 skip install';
  }
  if (dir.includes('..')) {
    return '.. not allowed';
  }
  if (!existsSync('.git')) {
    return ".git can't be found";
  }

  const internal = (x = '') => join(dir, '_', x);
  const { status, stderr } = spawnSync('git', ['config', 'core.hooksPath', `${dir}/_`]);
  if (status == null) {
    return 'git command not found';
  }
  if (status) {
    return '' + stderr;
  }

  rmSync(internal('husky.sh'), { force: true });
  mkdirSync(internal(), { recursive: true });
  writeFileSync(internal('.gitignore'), '*');
  writeFileSync(internal('h'), HOOK_SCRIPT, { mode: 0o755 });
  for (const hook of HOOKS) {
    writeFileSync(internal(hook), `#!/usr/bin/env sh\n. "$(dirname "$0")/h"`, { mode: 0o755 });
  }
  return '';
}

async function main() {
  const args = mri(process.argv.slice(3), {
    alias: { h: 'help' },
    boolean: ['help'],
  });

  if (args.help) {
    log((await getVitePlusHeader()) + '\n');
    log(helpMessage);
    return;
  }

  const result = install();
  if (result) {
    // Exit 0 on non-fatal conditions (no .git, HUSKY=0) — matches husky's behavior.
    // The "prepare" lifecycle runs during `npm install` in consumer projects too,
    // so it must not fail when .git doesn't exist.
    console.error(result);
  }
}

void main();
