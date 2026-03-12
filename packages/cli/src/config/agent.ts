import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';

import * as prompts from '@voidzero-dev/vite-plus-prompts';

import {
  detectAgents,
  getAgentById,
  hasExistingAgentInstructions,
  replaceMarkedAgentInstructionsSection,
  type AgentConfig,
  type McpConfigTarget,
} from '../utils/agent.js';
import { writeJsonFile, readJsonFile } from '../utils/json.js';
import { pkgRoot } from '../utils/path.js';

export interface AgentSetupSelection {
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

export async function resolveAgentSetup(
  root: string,
  interactive: boolean,
): Promise<AgentSetupSelection> {
  const detected = detectAgents(root);
  if (detected.length > 0 || !interactive) {
    return {
      instructionFilePath: detectInstructionFilePath(root, detected),
      agents: detected,
    };
  }
  return pickAgentWhenUndetected();
}

// --- Template reading ---

function readAgentPrompt(): string {
  return readFileSync(join(pkgRoot, 'AGENTS.md'), 'utf-8');
}

// --- Agent instructions injection ---

export { hasExistingAgentInstructions };

export function injectAgentBlock(root: string, filePath: string): void {
  const fullPath = join(root, filePath);
  const template = readAgentPrompt();

  if (existsSync(fullPath)) {
    const existing = readFileSync(fullPath, 'utf-8');
    const updated = replaceMarkedAgentInstructionsSection(existing, template);
    if (updated !== undefined) {
      if (updated !== existing) {
        writeFileSync(fullPath, updated);
        prompts.log.success(`Updated Vite+ instructions in ${filePath}`);
      } else {
        prompts.log.info(`${filePath} already has up-to-date Vite+ instructions`);
      }
    } else {
      // No markers found — append template
      const separator = existing.endsWith('\n') ? '\n' : '\n\n';
      writeFileSync(fullPath, existing + separator + template);
      prompts.log.success(`Added Vite+ instructions to ${filePath}`);
    }
  } else {
    writeFileSync(fullPath, template);
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

export function setupMcpConfig(root: string, selectedAgents: AgentConfig[]): void {
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

  const mcpAgents: { agent: AgentConfig; targets: McpConfigTarget[] }[] = [];
  const hintAgents: { agent: AgentConfig; hint: string }[] = [];

  for (const agent of selectedAgents) {
    if (agent.mcpConfig) {
      mcpAgents.push({ agent, targets: agent.mcpConfig });
    } else if (agent.mcpHint) {
      hintAgents.push({ agent, hint: agent.mcpHint });
    }
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
