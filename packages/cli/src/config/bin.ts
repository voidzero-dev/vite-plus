import { existsSync } from 'node:fs';
import { join, relative } from 'node:path';

import { parseConfigArgs } from '../../binding/index.js';
import { updateExistingAgentInstructions } from '../utils/agent.ts';
import { unwrapCliParseOutcome } from '../utils/cli-parse.ts';
import { findLegacyVitestAliasConfig } from '../utils/legacy-vitest-alias.ts';
import { defaultInteractive, promptGitHooks } from '../utils/prompts.ts';
import { errorMsg, log } from '../utils/terminal.ts';
import {
  install,
  isGitHooksEnvDisabled,
  isHooksUserDisabled,
  resolveHooksLocation,
} from './hooks.ts';

async function main() {
  const args = unwrapCliParseOutcome(parseConfigArgs(process.argv.slice(3)));
  const dir = args.hooksDir;
  const skipHooks = args.hooks === false;
  const skipAgent = args.agent === false;
  const interactive = defaultInteractive();
  const lifecycleEvent = process.env.npm_lifecycle_event;
  const isLifecycleScript = lifecycleEvent === 'prepare' || lifecycleEvent === 'postinstall';
  const root = process.cwd();

  const staleVitestAliasConfig = findLegacyVitestAliasConfig(root);
  if (staleVitestAliasConfig) {
    const relativeConfigPath = relative(root, staleVitestAliasConfig) || 'package.json';
    errorMsg(
      `Found a stale Vitest alias in ${relativeConfigPath} that points to the removed \`@voidzero-dev/vite-plus-test\` package. Run \`vp migrate\` to update the project before installing dependencies.`,
    );
    process.exit(1);
  }

  // --- Step 1: Hooks setup ---
  // Prefer CLI flag, then last-used dir from local git config, then default.
  // Check environment opt-outs before the Git-backed location lookup.
  if (!skipHooks && isGitHooksEnvDisabled()) {
    log('skip install (git hooks disabled)');
  } else if (!skipHooks) {
    const location = resolveHooksLocation(dir);
    if ('isError' in location) {
      if (location.message) {
        log(location.message);
      }
      if (location.isError) {
        process.exit(1);
      }
    } else {
      const isFirstHooksRun = !existsSync(join(location.baseDir, location.dir, '_', 'pre-commit'));

      let shouldSetupHooks = true;
      if (isHooksUserDisabled()) {
        // Honor `vp hooks disable` without re-prompting (option A).
        log('skip install (hooks disabled; run `vp hooks enable` to re-enable)');
        shouldSetupHooks = false;
      } else if (interactive && isFirstHooksRun && !dir && !isLifecycleScript) {
        // Explicit directories and lifecycle scripts already opt in.
        shouldSetupHooks = await promptGitHooks({
          interactive,
          message: 'Install the Git hook dispatcher for this project?',
        });
      }

      if (shouldSetupHooks) {
        const { message, isError } = install(dir);
        if (message) {
          log(message);
          if (isError) {
            process.exit(1);
          }
        }
      }
    }
  }

  // --- Step 2: Update agent instructions if Vite+ header exists and is outdated ---
  if (!skipAgent) {
    updateExistingAgentInstructions(root);
  }
}

void main();
