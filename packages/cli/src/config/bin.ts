// Unified `vp config` command — merges the old `vp prepare` (hooks setup) and
// `vp init` (agent integration) into a single entry point.
//
// Interactive mode (TTY, no CI): prompts on first run, updates silently after.
// Non-interactive mode (scripts.prepare, CI, piped): runs everything by default.

import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join, relative } from 'node:path';

import * as prompts from '@voidzero-dev/vite-plus-prompts';
import mri from 'mri';

import { vitePlusHeader } from '../../binding/index.js';
import {
  detectAgents,
  getAgentById,
  type AgentConfig,
  type McpConfigTarget,
} from '../utils/agent.js';
import { renderCliDoc } from '../utils/help.js';
import { writeJsonFile, readJsonFile } from '../utils/json.js';
import { pkgRoot } from '../utils/path.js';
import { defaultInteractive, promptGitHooks } from '../utils/prompts.js';
import { linkSkillsForSpecificAgents } from '../utils/skills.js';
import { log } from '../utils/terminal.js';

// ---------------------------------------------------------------------------
// Hooks setup (from prepare/bin.ts)
// ---------------------------------------------------------------------------

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

interface InstallResult {
  message: string;
  isError: boolean;
}

function install(dir = '.vite-hooks'): InstallResult {
  if (process.env.HUSKY === '0') {
    return { message: 'HUSKY=0 skip install', isError: false };
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

// ---------------------------------------------------------------------------
// Agent setup (from init/bin.ts)
// ---------------------------------------------------------------------------

interface AgentSetupSelection {
  instructionFilePath: 'CLAUDE.md' | 'AGENTS.md';
  agents: AgentConfig[];
}

function detectInstructionFilePath(
  root: string,
  agentConfigs: AgentConfig[],
): 'CLAUDE.md' | 'AGENTS.md' {
  if (agentConfigs.some((a) => a.skillsDir === '.claude/skills')) {
    return 'CLAUDE.md';
  }
  if (existsSync(join(root, 'CLAUDE.md'))) {
    return 'CLAUDE.md';
  }
  return 'AGENTS.md';
}

async function pickAgentWhenUndetected(): Promise<AgentSetupSelection> {
  const choice = await prompts.select({
    message: 'Could not detect your coding agent. Which one are you using?',
    options: [
      { value: 'claude-code', label: 'Claude Code' },
      { value: 'cursor', label: 'Cursor' },
      { value: 'codex', label: 'Codex' },
      { value: 'gemini-cli', label: 'Gemini CLI' },
      { value: 'generic', label: 'Generic' },
    ],
  });
  if (prompts.isCancel(choice)) {
    prompts.cancel('Setup cancelled.');
    process.exit(0);
  }

  if (choice === 'generic') {
    return {
      instructionFilePath: 'AGENTS.md',
      agents: [],
    };
  }

  const selected = getAgentById(choice);
  if (!selected) {
    return {
      instructionFilePath: 'AGENTS.md',
      agents: [],
    };
  }

  return {
    instructionFilePath: choice === 'claude-code' ? 'CLAUDE.md' : 'AGENTS.md',
    agents: [selected],
  };
}

async function resolveAgentSetup(root: string): Promise<AgentSetupSelection> {
  const detected = detectAgents(root);
  if (detected.length > 0) {
    return {
      instructionFilePath: detectInstructionFilePath(root, detected),
      agents: detected,
    };
  }
  return pickAgentWhenUndetected();
}

// --- Version and template reading ---

function getOwnVersion(): string {
  const pkg = JSON.parse(readFileSync(join(pkgRoot, 'package.json'), 'utf-8'));
  if (typeof pkg.version !== 'string') {
    throw new Error('vite-plus package.json is missing a "version" field');
  }
  return pkg.version;
}

function readAgentPrompt(): string {
  return readFileSync(join(pkgRoot, 'AGENTS.md'), 'utf-8');
}

// --- Versioned injection ---

const MARKER_OPEN_RE = /<!--injected-by-vite-plus-v([\w.+-]+)-->/;
const MARKER_CLOSE = '<!--/injected-by-vite-plus-->';
const MARKER_BLOCK_RE =
  /<!--injected-by-vite-plus-v[\w.+-]+-->\n[\s\S]*?<!--\/injected-by-vite-plus-->/;

function hasExistingAgentInstructions(root: string): boolean {
  for (const file of ['AGENTS.md', 'CLAUDE.md']) {
    const fullPath = join(root, file);
    if (existsSync(fullPath)) {
      const content = readFileSync(fullPath, 'utf-8');
      if (MARKER_OPEN_RE.test(content)) {
        return true;
      }
    }
  }
  return false;
}

function injectAgentBlock(root: string, filePath: string): void {
  const fullPath = join(root, filePath);
  const version = getOwnVersion();
  const promptContent = readAgentPrompt();
  const openMarker = `<!--injected-by-vite-plus-v${version}-->`;
  const block = `${openMarker}\n${promptContent}\n${MARKER_CLOSE}`;

  if (existsSync(fullPath)) {
    const existing = readFileSync(fullPath, 'utf-8');
    const match = existing.match(MARKER_OPEN_RE);
    if (match) {
      if (match[1] === version) {
        prompts.log.info(`${filePath} already has Vite+ instructions (v${version})`);
        return;
      }
      // Replace existing block with updated version
      const updated = existing.replace(MARKER_BLOCK_RE, block);
      if (updated === existing) {
        // Closing marker is missing or malformed — append fresh block
        const separator = existing.endsWith('\n') ? '\n' : '\n\n';
        writeFileSync(fullPath, existing + separator + block + '\n');
        prompts.log.warn(`Existing Vite+ block in ${filePath} was malformed; appended fresh block`);
      } else {
        writeFileSync(fullPath, updated);
        prompts.log.success(
          `Updated Vite+ instructions in ${filePath} (v${match[1]} → v${version})`,
        );
      }
    } else {
      // Append block to end of file
      const separator = existing.endsWith('\n') ? '\n' : '\n\n';
      writeFileSync(fullPath, existing + separator + block + '\n');
      prompts.log.success(`Added Vite+ instructions to ${filePath}`);
    }
  } else {
    writeFileSync(fullPath, block + '\n');
    prompts.log.success(`Created ${filePath} with Vite+ instructions`);
  }
}

// --- MCP config ---

function writeMcpConfigForTarget(root: string, target: McpConfigTarget): void {
  const fullPath = join(root, target.filePath);
  let existing: Record<string, any> = {};
  if (existsSync(fullPath)) {
    try {
      existing = readJsonFile(fullPath);
    } catch {
      prompts.log.warn(
        `Could not parse ${target.filePath} — skipping MCP config. Please add the config manually.`,
      );
      return;
    }
  }

  if (!existing[target.rootKey]) {
    existing[target.rootKey] = {};
  }

  if (existing[target.rootKey]['vite-plus']) {
    prompts.log.info(`${target.filePath} already has vite-plus MCP config`);
    return;
  }

  existing[target.rootKey]['vite-plus'] = {
    command: 'npx',
    args: ['vp', 'mcp'],
    ...target.extraFields,
  };

  mkdirSync(dirname(fullPath), { recursive: true });
  writeJsonFile(fullPath, existing);
  prompts.log.success(`Added vite-plus MCP server to ${target.filePath}`);
}

function pickMcpTarget(root: string, targets: McpConfigTarget[]): McpConfigTarget {
  if (targets.length === 1) {
    return targets[0];
  }
  return targets.find((t) => existsSync(join(root, t.filePath))) ?? targets[0];
}

function setupMcpConfig(root: string, selectedAgents: AgentConfig[]): void {
  const mcpAgents: { agent: AgentConfig; targets: McpConfigTarget[] }[] = [];
  const hintAgents: { agent: AgentConfig; hint: string }[] = [];

  for (const agent of selectedAgents) {
    if (agent.mcpConfig) {
      mcpAgents.push({ agent, targets: agent.mcpConfig });
    } else if (agent.mcpHint) {
      hintAgents.push({ agent, hint: agent.mcpHint });
    }
  }

  if (selectedAgents.length === 0) {
    prompts.note(
      JSON.stringify(
        {
          'vite-plus': {
            command: 'npx',
            args: ['vp', 'mcp'],
          },
        },
        null,
        2,
      ),
      'Add this MCP server config to your agent',
    );
    return;
  }

  // Print hints for agents without project-level config
  for (const { agent, hint } of hintAgents) {
    prompts.log.info(`${agent.displayName}: ${hint}`);
  }

  // Write config for agents with project-level support
  for (const { agent, targets } of mcpAgents) {
    const target = pickMcpTarget(root, targets);
    prompts.log.info(`${agent.displayName} MCP target: ${target.filePath}`);
    writeMcpConfigForTarget(root, target);
  }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

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
          rows: [{ label: 'HUSKY=0', description: 'Skip hook installation' }],
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
    shouldSetupHooks = await promptGitHooks({ interactive, hooks: undefined });
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

  // --- Step 2: Agent setup (skipped with --hooks-only) ---
  if (!hooksOnly) {
    const isFirstAgentRun = !hasExistingAgentInstructions(root);

    let agentSetup: AgentSetupSelection;
    if (interactive && isFirstAgentRun) {
      agentSetup = await resolveAgentSetup(root);
    } else {
      // Non-interactive or subsequent run: auto-detect
      const detected = detectAgents(root);
      agentSetup = {
        instructionFilePath: detectInstructionFilePath(root, detected),
        agents: detected,
      };
    }

    injectAgentBlock(root, agentSetup.instructionFilePath);
    setupMcpConfig(root, agentSetup.agents);
    if (agentSetup.agents.length > 0) {
      linkSkillsForSpecificAgents(root, agentSetup.agents);
    }
  }
}

void main();
