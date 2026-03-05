// Unified `vp config` command — merges the old `vp prepare` (hooks setup) and
// `vp init` (agent integration) into a single entry point.
//
// Interactive mode (TTY, no CI): prompts on first run, updates silently after.
// Non-interactive mode (scripts.prepare, CI, piped): runs everything by default.

import { existsSync } from 'node:fs';
import { join } from 'node:path';

import mri from 'mri';

import { vitePlusHeader } from '../../binding/index.js';
import { renderCliDoc } from '../utils/help.js';
import { defaultInteractive, promptGitHooks } from '../utils/prompts.js';
import { linkSkillsForSpecificAgents } from '../utils/skills.js';
import { log } from '../utils/terminal.js';
import {
  resolveAgentSetup,
  hasExistingAgentInstructions,
  injectAgentBlock,
  setupMcpConfig,
} from './agent.js';
import { install } from './hooks.js';

async function main() {
  const args = mri(process.argv.slice(3), {
    boolean: ['help', 'hooks-only'],
    string: ['hooks-dir'],
    alias: { h: 'help' },
  });

  if (args.help) {
    const helpMessage = renderCliDoc({
      usage: 'vp config [OPTIONS]',
      summary: 'Configure Vite+ for the current project (hooks + agent integration).',
      sections: [
        {
          title: 'Options',
          rows: [
            {
              label: '--hooks-dir <path>',
              description: 'Custom hooks directory (default: .vite-hooks)',
            },
            { label: '-h, --help', description: 'Show this help message' },
          ],
        },
        {
          title: 'Environment',
          rows: [{ label: 'VITE_GIT_HOOKS=0', description: 'Skip hook installation' }],
        },
      ],
    });
    log(vitePlusHeader() + '\n');
    log(helpMessage);
    return;
  }

  const dir = args['hooks-dir'] as string | undefined;
  const hooksOnly = args['hooks-only'] as boolean;
  const interactive = defaultInteractive();
  const root = process.cwd();

  // --- Step 1: Hooks setup ---
  const hooksDir = dir ?? '.vite-hooks';
  const isFirstHooksRun = !existsSync(join(root, hooksDir, 'pre-commit'));

  let shouldSetupHooks = true;
  if (interactive && isFirstHooksRun && !dir) {
    // --hooks-dir implies agreement; only prompt when using default dir on first run
    shouldSetupHooks = await promptGitHooks({ interactive });
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

  // --- Step 2: Agent setup (skipped with --hooks-only or during prepare lifecycle) ---
  if (!hooksOnly && process.env.npm_lifecycle_event !== 'prepare') {
    const isFirstAgentRun = !hasExistingAgentInstructions(root);
    const agentSetup = await resolveAgentSetup(root, interactive && isFirstAgentRun);

    injectAgentBlock(root, agentSetup.instructionFilePath);
    setupMcpConfig(root, agentSetup.agents);
    if (agentSetup.agents.length > 0) {
      linkSkillsForSpecificAgents(root, agentSetup.agents);
    }
  }
}

void main();
