import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';

import * as prompts from '@voidzero-dev/vite-plus-prompts';

import {
  detectAgents,
  getAgentById,
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

export function hasExistingAgentInstructions(root: string): boolean {
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

export function injectAgentBlock(root: string, filePath: string): void {
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
