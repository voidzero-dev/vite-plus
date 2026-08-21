// Runs staged linters on staged files using the lint-staged programmatic API.
// Bundled by rolldown — no runtime dependency needed in user projects.
//
// Reads the "staged" key from vite.config.ts via resolveConfig() and passes it
// to lint-staged as an explicit config object.  Exits with a warning if no
// staged config is found.
//
// We use the programmatic API instead of importing lint-staged/bin because
// lint-staged's dependency tree includes CJS modules that use require('node:events')
// etc., which breaks when bundled to ESM format by rolldown.

import lintStaged from 'lint-staged';
import type { Options } from 'lint-staged';

import { parseStagedArgs } from '../../binding/index.js';
import { resolveViteConfig } from '../resolve-vite-config.ts';
import { renderCliDoc } from '../utils/help.ts';
import { errorMsg, log, printHeader } from '../utils/terminal.ts';

const parsedArgs = parseStagedArgs(process.argv.slice(3));

if (parsedArgs.status === 'help') {
  const helpMessage = renderCliDoc({
    usage: 'vp staged [options]',
    summary: 'Run linters on staged files using staged config from vite.config.ts.',
    documentationUrl: 'https://viteplus.dev/guide/commit-hooks',
    sections: [
      {
        title: 'Options',
        rows: [
          {
            label: '--allow-empty',
            description: 'Allow empty commits when tasks revert all staged changes',
          },
          {
            label: '-p, --concurrent [number|boolean]',
            description: 'Number of tasks to run concurrently, or false for serial',
          },
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
          { label: '--no-stash', description: 'Disable the backup stash' },
          { label: '-q, --quiet', description: 'Disable console output' },
          { label: '-r, --relative', description: 'Pass filepaths relative to cwd to tasks' },
          { label: '--revert', description: 'Revert to original state in case of errors' },
          { label: '-v, --verbose', description: 'Show task output even when tasks succeed' },
          { label: '-h, --help', description: 'Show this help message' },
        ],
      },
    ],
  });
  printHeader();
  log(helpMessage);
} else if (parsedArgs.status === 'error') {
  printHeader();
  errorMsg(parsedArgs.error.message.replace(/^error:\s*/, ''));
  process.exit(1);
} else {
  const args = parsedArgs.value;

  const options: Options = {};

  // Boolean flags — only include if explicitly set
  if (args.allowEmpty != null) {
    options.allowEmpty = args.allowEmpty;
  }
  if (args.debug != null) {
    options.debug = args.debug;
  }
  if (args.continueOnError != null) {
    options.continueOnError = args.continueOnError;
  }
  if (args.failOnChanges != null) {
    options.failOnChanges = args.failOnChanges;
  }
  if (args.hidePartiallyStaged != null) {
    options.hidePartiallyStaged = args.hidePartiallyStaged;
  }
  if (args.hideUnstaged != null) {
    options.hideUnstaged = args.hideUnstaged;
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

  // Read "staged" from vite.config.ts and pass it as an inline config object to lint-staged.
  let stagedConfig;
  try {
    const viteConfig = await resolveViteConfig(args.cwd ?? process.cwd());
    stagedConfig = viteConfig.staged;
  } catch (err) {
    // Surface real errors (syntax errors, missing imports, etc.)
    // instead of masking them as "no config found"
    const message = err instanceof Error ? err.message : String(err);
    log(`Failed to load vite.config: ${message}`);
    process.exit(1);
  }
  if (stagedConfig) {
    options.config = stagedConfig;
  } else {
    printHeader();
    errorMsg('No "staged" config found in vite.config.ts. Please add a staged config:');
    log('');
    log('  // vite.config.ts');
    log('  export default defineConfig({');
    log("    staged: { '*': 'vp check --fix' },");
    log('  });');
    process.exit(1);
  }
  if (args.cwd != null) {
    options.cwd = args.cwd;
  }
  if (args.diff != null) {
    options.diff = args.diff;
  }
  if (args.diffFilter != null) {
    options.diffFilter = args.diffFilter;
  }
  if (args.concurrent != null) {
    options.concurrent = args.concurrent;
  }

  const success = await lintStaged(options);
  process.exit(success ? 0 : 1);
}
