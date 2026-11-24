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

import type { Options } from 'lint-staged';
import lintStaged from 'lint-staged';

import { parseStagedArgs } from '../../binding/index.js';
import { resolveViteConfig } from '../resolve-vite-config.ts';
import { unwrapCliParseOutcome } from '../utils/cli-parse.ts';
import { errorMsg, log, printHeader } from '../utils/terminal.ts';
import { createStagedLogger } from './logger.ts';

const args = unwrapCliParseOutcome(parseStagedArgs(process.argv.slice(3)));
const options: Options = {};

// Boolean flags — only include if explicitly set
if (args.allowEmpty !== null && args.allowEmpty !== undefined) {
  options.allowEmpty = args.allowEmpty;
}
if (args.debug !== null && args.debug !== undefined) {
  options.debug = args.debug;
}
if (args.continueOnError !== null && args.continueOnError !== undefined) {
  options.continueOnError = args.continueOnError;
}
if (args.failOnChanges !== null && args.failOnChanges !== undefined) {
  options.failOnChanges = args.failOnChanges;
}
if (args.hidePartiallyStaged !== null && args.hidePartiallyStaged !== undefined) {
  options.hidePartiallyStaged = args.hidePartiallyStaged;
}
if (args.hideUnstaged !== null && args.hideUnstaged !== undefined) {
  options.hideUnstaged = args.hideUnstaged;
}
if (args.quiet !== null && args.quiet !== undefined) {
  options.quiet = args.quiet;
}
if (args.relative !== null && args.relative !== undefined) {
  options.relative = args.relative;
}
if (args.revert !== null && args.revert !== undefined) {
  options.revert = args.revert;
}
if (args.stash !== null && args.stash !== undefined) {
  options.stash = args.stash;
}
if (args.verbose !== null && args.verbose !== undefined) {
  options.verbose = args.verbose;
}

// Read "staged" from vite.config.ts and pass it as an inline config object to lint-staged.
let stagedConfig;
try {
  const viteConfig = await resolveViteConfig(args.cwd ?? process.cwd());
  stagedConfig = viteConfig.staged;
} catch (error) {
  // Surface real errors (syntax errors, missing imports, etc.)
  // instead of masking them as "no config found"
  const message = error instanceof Error ? error.message : String(error);
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
if (args.cwd !== null && args.cwd !== undefined) {
  options.cwd = args.cwd;
}
if (args.diff !== null && args.diff !== undefined) {
  options.diff = args.diff;
}
if (args.diffFilter !== null && args.diffFilter !== undefined) {
  options.diffFilter = args.diffFilter;
}
if (args.concurrent !== null && args.concurrent !== undefined) {
  options.concurrent = args.concurrent;
}

const success = await lintStaged(
  options,
  options.debug || options.quiet ? undefined : createStagedLogger(),
);
process.exit(success ? 0 : 1);
