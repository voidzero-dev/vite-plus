import mri from 'mri';

import { DEFAULT_HOOKS_DIR, disable, enable, status } from '../config/hooks.ts';
import { renderCliDoc } from '../utils/help.ts';
import { log, printHeader } from '../utils/terminal.ts';
import { unexpectedHooksArgsError } from './args.ts';

const SUBCOMMANDS = ['enable', 'disable', 'status'] as const;
type Subcommand = (typeof SUBCOMMANDS)[number];

function isSubcommand(value: string | undefined): value is Subcommand {
  return !!value && (SUBCOMMANDS as readonly string[]).includes(value);
}

function printHelp(): void {
  const helpMessage = renderCliDoc({
    usage: 'vp hooks <COMMAND> [OPTIONS]',
    summary: 'Manage the Vite+ Git hook dispatcher for this repository.',
    documentationUrl: 'https://viteplus.dev/guide/commit-hooks',
    sections: [
      {
        title: 'Commands',
        rows: [
          {
            label: 'enable',
            description: 'Install or refresh the hook dispatcher (sets core.hooksPath)',
          },
          {
            label: 'disable',
            description: 'Disable hooks: unset core.hooksPath, remove <dir>/_, persist preference',
          },
          {
            label: 'status',
            description: 'Show preference, core.hooksPath, and dispatcher state',
          },
        ],
      },
      {
        title: 'Options',
        rows: [
          {
            label: '--hooks-dir <path>',
            description: `Custom hooks directory (default: ${DEFAULT_HOOKS_DIR}, or last used)`,
          },
          { label: '-h, --help', description: 'Show this help message' },
        ],
      },
      {
        title: 'Environment',
        rows: [
          {
            label: 'VP_GIT_HOOKS=0',
            description: 'Skip dispatcher install in enable (and skip hooks at commit time)',
          },
        ],
      },
      {
        title: 'Examples',
        lines: [
          '  vp hooks enable',
          '  vp hooks enable --hooks-dir .custom-hooks',
          '  vp hooks disable',
          '  vp hooks status',
        ],
      },
    ],
  });
  printHeader();
  log(helpMessage);
}

function applyResult(result: { message: string; isError: boolean }): void {
  if (result.message) {
    log(result.message);
  }
  if (result.isError) {
    process.exit(1);
  }
}

async function main() {
  // argv: [node, bin, hooks, <subcommand>?, ...flags]
  const raw = process.argv.slice(3);
  const first = raw[0];
  const wantsHelp = raw.includes('-h') || raw.includes('--help');

  // `vp hooks` / `vp hooks -h` → top-level help
  if (!first || first === '-h' || first === '--help') {
    printHelp();
    return;
  }

  if (!isSubcommand(first)) {
    log(`Unknown hooks command "${first}". Expected one of: ${SUBCOMMANDS.join(', ')}`);
    process.exit(1);
  }

  const subcommand = first;
  const args = mri(raw.slice(1), {
    boolean: ['help'],
    string: ['hooks-dir'],
    alias: { h: 'help' },
  });

  if (args.help || wantsHelp) {
    printHelp();
    return;
  }

  const unexpected = unexpectedHooksArgsError(args);
  if (unexpected) {
    log(unexpected);
    process.exit(1);
  }

  const dirFlag = args['hooks-dir'] as string | undefined;

  switch (subcommand) {
    case 'enable':
      applyResult(enable(dirFlag));
      return;
    case 'disable':
      applyResult(disable(dirFlag));
      return;
    case 'status':
      applyResult(status(dirFlag));
      return;
  }
}

void main();
