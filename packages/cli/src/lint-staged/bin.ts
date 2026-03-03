// Runs lint-staged on staged files using the programmatic API.
// Bundled by rolldown — no runtime dependency needed in user projects.
//
// We use the programmatic API instead of importing lint-staged/bin because
// lint-staged's dependency tree includes CJS modules that use require('node:events')
// etc., which breaks when bundled to ESM format by rolldown.
import lintStaged from 'lint-staged';
import type { Options } from 'lint-staged';
import mri from 'mri';

import { renderCliDoc } from '../utils/help.js';
import { getVitePlusHeader, log } from '../utils/terminal.js';

const args = mri(process.argv.slice(3), {
  alias: {
    h: 'help',
    p: 'concurrent',
    c: 'config',
    d: 'debug',
    q: 'quiet',
    r: 'relative',
    v: 'verbose',
  },
  boolean: [
    'help',
    'allow-empty',
    'debug',
    'continue-on-error',
    'fail-on-changes',
    'hide-partially-staged',
    'hide-unstaged',
    'quiet',
    'relative',
    'revert',
    'stash',
    'verbose',
  ],
  string: ['concurrent', 'config', 'cwd', 'diff', 'diff-filter', 'max-arg-length'],
});

if (args.help) {
  const helpMessage = renderCliDoc({
    usage: 'vp lint-staged [options]',
    summary: 'Run linters on staged files.',
    sections: [
      {
        title: 'Options',
        rows: [
          {
            label: '--allow-empty',
            description: 'Allow empty commits when tasks revert all staged changes',
          },
          {
            label: '-p, --concurrent <number|boolean>',
            description: 'Number of tasks to run concurrently, or false for serial',
          },
          { label: '-c, --config <path>', description: 'Path to configuration file' },
          {
            label: '--continue-on-error',
            description: 'Run all tasks to completion even if one fails',
          },
          { label: '--cwd <path>', description: 'Working directory to run all tasks in' },
          { label: '-d, --debug', description: 'Enable debug output' },
          {
            label: '--diff <string>',
            description: 'Override the default --staged flag of git diff',
          },
          {
            label: '--diff-filter <string>',
            description: 'Override the default --diff-filter=ACMR flag of git diff',
          },
          {
            label: '--fail-on-changes',
            description: 'Fail with exit code 1 when tasks modify tracked files',
          },
          {
            label: '--hide-partially-staged',
            description: 'Hide unstaged changes from partially staged files',
          },
          {
            label: '--hide-unstaged',
            description: 'Hide all unstaged changes before running tasks',
          },
          { label: '--max-arg-length <number>', description: 'Maximum argument string length' },
          { label: '--no-stash', description: 'Disable the backup stash' },
          { label: '-q, --quiet', description: 'Disable lint-staged console output' },
          { label: '-r, --relative', description: 'Pass filepaths relative to cwd to tasks' },
          { label: '--revert', description: 'Revert to original state in case of errors' },
          { label: '-v, --verbose', description: 'Show task output even when tasks succeed' },
          { label: '-h, --help', description: 'Show this help message' },
        ],
      },
    ],
  });
  log((await getVitePlusHeader()) + '\n');
  log(helpMessage);
} else {
  const options: Options = {};

  // Boolean flags — only include if explicitly set
  if (args['allow-empty'] != null) {
    options.allowEmpty = args['allow-empty'];
  }
  if (args.debug != null) {
    options.debug = args.debug;
  }
  if (args['continue-on-error'] != null) {
    options.continueOnError = args['continue-on-error'];
  }
  if (args['fail-on-changes'] != null) {
    options.failOnChanges = args['fail-on-changes'];
  }
  if (args['hide-partially-staged'] != null) {
    options.hidePartiallyStaged = args['hide-partially-staged'];
  }
  if (args['hide-unstaged'] != null) {
    options.hideUnstaged = args['hide-unstaged'];
  }
  if (args.quiet != null) {
    options.quiet = args.quiet;
  }
  if (args.relative != null) {
    options.relative = args.relative;
  }
  if (args.revert != null) {
    options.revert = args.revert;
  }
  if (args.stash != null) {
    options.stash = args.stash;
  }
  if (args.verbose != null) {
    options.verbose = args.verbose;
  }

  // String flags
  if (args.config != null) {
    if (args.config === '-') {
      // stdin mode: read JSON config from stdin (matches lint-staged's -c - behavior)
      const chunks: Buffer[] = [];
      for await (const chunk of process.stdin) {
        chunks.push(chunk as Buffer);
      }
      const stdinContent = Buffer.concat(chunks).toString('utf8').trim();
      if (stdinContent) {
        options.config = JSON.parse(stdinContent);
      }
    } else {
      options.configPath = args.config;
    }
  }
  if (args.cwd != null) {
    options.cwd = args.cwd;
  }
  if (args.diff != null) {
    options.diff = args.diff;
  }
  if (args['diff-filter'] != null) {
    options.diffFilter = args['diff-filter'];
  }

  // Parsed flags: concurrent → boolean | number
  if (args.concurrent != null) {
    const val = args.concurrent;
    if (val === 'true') {
      options.concurrent = true;
    } else if (val === 'false') {
      options.concurrent = false;
    } else {
      const num = Number(val);
      options.concurrent = Number.isNaN(num) ? true : num;
    }
  }

  // Parsed flags: max-arg-length → number
  if (args['max-arg-length'] != null) {
    const num = Number(args['max-arg-length']);
    if (!Number.isNaN(num)) {
      options.maxArgLength = num;
    }
  }

  const success = await lintStaged(options);
  process.exit(success ? 0 : 1);
}
