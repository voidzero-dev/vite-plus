// Built-in husky-compatible install logic — a reimplementation of husky v9's
// install function. husky itself is not bundled as a dependency.
//
// Why reimplementation instead of bundling husky?
// husky v9's install function uses `new URL('husky', import.meta.url)` to
// resolve and copy its shell script (the hook dispatcher). When bundled by
// rolldown, `import.meta.url` points to the bundled output directory, not the
// original `node_modules/husky/` directory, so the shell script file cannot be
// found. Rather than working around this with asset copying, we inline the
// equivalent shell script as a string constant (HOOK_SCRIPT) and write it
// directly via writeFileSync.

import { spawnSync } from 'node:child_process';
import {
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  rmSync,
  unlinkSync,
  writeFileSync,
} from 'node:fs';
import { join, relative } from 'node:path';

import mri from 'mri';

import { vitePlusHeader } from '../../binding/index.js';
import { renderCliDoc } from '../utils/help.js';
import { log } from '../utils/terminal.js';

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

d=$(dirname "$(dirname "$(dirname "$0")")")
export PATH="$d/node_modules/.bin:$PATH"
sh -e "$s" "$@"
c=$?

[ $c != 0 ] && echo "husky - $n script failed (code $c)"
[ $c = 127 ] && echo "husky - command not found in PATH=$PATH"
exit $c`;

const HUSKY_BOOTSTRAP_PATTERN = /^\.\s+".*husky\.sh"/;

/**
 * Strip the stale `. "…/husky.sh"` bootstrap line from every user-defined
 * hook file in the husky directory. Called during `vp prepare` so that repos
 * manually switching from Husky don't break on the next commit.
 */
function stripHuskyBootstrapFromHooks(huskyDir: string): void {
  if (!existsSync(huskyDir)) {
    return;
  }
  const entries = readdirSync(huskyDir, { withFileTypes: true });
  for (const entry of entries) {
    if (!entry.isFile()) {
      continue;
    }
    const hookPath = join(huskyDir, entry.name);
    const content = readFileSync(hookPath, 'utf8');
    const lines = content.split('\n');
    const filtered = lines.filter((line) => !HUSKY_BOOTSTRAP_PATTERN.test(line.trim()));
    if (filtered.length === lines.length) {
      continue;
    }
    const newContent = filtered.join('\n').trim();
    if (newContent.length === 0) {
      unlinkSync(hookPath);
    } else {
      writeFileSync(hookPath, `${newContent}\n`);
    }
  }
}

interface InstallResult {
  message: string;
  isError: boolean;
}

function install(dir = '.husky'): InstallResult {
  if (process.env.HUSKY === '0') {
    return { message: 'HUSKY=0 skip install', isError: false };
  }
  if (dir.includes('..')) {
    return { message: '.. not allowed', isError: false };
  }
  const topResult = spawnSync('git', ['rev-parse', '--show-toplevel']);
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
  stripHuskyBootstrapFromHooks(dir);
  mkdirSync(internal(), { recursive: true });
  writeFileSync(internal('.gitignore'), '*');
  writeFileSync(internal('h'), HOOK_SCRIPT, { mode: 0o755 });
  for (const hook of HOOKS) {
    writeFileSync(internal(hook), `#!/usr/bin/env sh\n. "$(dirname "$0")/h"`, { mode: 0o755 });
  }
  return { message: '', isError: false };
}

async function main() {
  const args = mri(process.argv.slice(3), {
    alias: { h: 'help' },
    boolean: ['help'],
  });

  if (args.help) {
    const helpMessage = renderCliDoc({
      usage: 'vp prepare',
      summary: 'Set up Git hooks for the project.',
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
    log(vitePlusHeader() + '\n');
    log(helpMessage);
    return;
  }

  const dir = args._[0] as string | undefined;
  const { message, isError } = install(dir);
  if (message) {
    log(message);
    if (isError) {
      process.exit(1);
    }
  }
}

void main();
