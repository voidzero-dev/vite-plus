import fs from 'node:fs';
import fsPromises from 'node:fs/promises';
import path from 'node:path';

import * as prompts from '@voidzero-dev/vite-plus-prompts';

import { pkgRoot } from './path.js';

// --- Interfaces ---

export interface McpConfigTarget {
  /** Config file path relative to project root, e.g. ".claude/settings.json" */
  filePath: string;
  /** JSON key that holds MCP server entries, e.g. "mcpServers" or "servers" */
  rootKey: string;
  /** Extra fields merged into the server entry, e.g. { type: "stdio" } for VS Code */
  extraFields?: Record<string, string>;
}

export interface AgentConfig {
  displayName: string;
  skillsDir: string;
  detect: (root: string) => boolean;
  /** Project-level config files where MCP server entries can be auto-written */
  mcpConfig?: McpConfigTarget[];
  /** Fallback hint printed when the agent has no project-level config support */
  mcpHint?: string;
}

// --- Agent registry ---

const DEFAULT_MCP_HINT =
  "Run `npx vp mcp` — this starts a stdio MCP server. See your agent's docs for how to add a local MCP server.";

const agents: Record<string, AgentConfig> = {
  'claude-code': {
    displayName: 'Claude Code',
    skillsDir: '.claude/skills',
    detect: (root) =>
      fs.existsSync(path.join(root, '.claude')) || fs.existsSync(path.join(root, 'CLAUDE.md')),
    mcpConfig: [
      { filePath: '.claude/settings.json', rootKey: 'mcpServers' },
      { filePath: '.claude/settings.local.json', rootKey: 'mcpServers' },
    ],
  },
  amp: {
    displayName: 'Amp',
    skillsDir: '.agents/skills',
    detect: (root) => fs.existsSync(path.join(root, '.amp')),
    mcpHint: DEFAULT_MCP_HINT,
  },
  codex: {
    displayName: 'Codex',
    skillsDir: '.agents/skills',
    detect: (root) => fs.existsSync(path.join(root, '.codex')),
    mcpHint: 'codex mcp add vite-plus -- npx vp mcp',
  },
  cursor: {
    displayName: 'Cursor',
    skillsDir: '.agents/skills',
    detect: (root) => fs.existsSync(path.join(root, '.cursor')),
    mcpConfig: [{ filePath: '.cursor/mcp.json', rootKey: 'mcpServers' }],
  },
  windsurf: {
    displayName: 'Windsurf',
    skillsDir: '.windsurf/skills',
    detect: (root) => fs.existsSync(path.join(root, '.windsurf')),
    mcpConfig: [{ filePath: '.windsurf/mcp.json', rootKey: 'mcpServers' }],
  },
  'gemini-cli': {
    displayName: 'Gemini CLI',
    skillsDir: '.agents/skills',
    detect: (root) => fs.existsSync(path.join(root, '.gemini')),
    mcpHint: 'gemini mcp add vite-plus -- npx vp mcp',
  },
  'github-copilot': {
    displayName: 'GitHub Copilot',
    skillsDir: '.agents/skills',
    detect: (root) =>
      fs.existsSync(path.join(root, '.github', 'copilot-instructions.md')) ||
      fs.existsSync(path.join(root, '.vscode', 'mcp.json')),
    mcpConfig: [
      { filePath: '.vscode/mcp.json', rootKey: 'servers', extraFields: { type: 'stdio' } },
    ],
  },
  cline: {
    displayName: 'Cline',
    skillsDir: '.cline/skills',
    detect: (root) => fs.existsSync(path.join(root, '.cline')),
    mcpHint: DEFAULT_MCP_HINT,
  },
  roo: {
    displayName: 'Roo Code',
    skillsDir: '.roo/skills',
    detect: (root) => fs.existsSync(path.join(root, '.roo')),
    mcpConfig: [{ filePath: '.roo/mcp.json', rootKey: 'mcpServers' }],
  },
  kilo: {
    displayName: 'Kilo Code',
    skillsDir: '.kilocode/skills',
    detect: (root) => fs.existsSync(path.join(root, '.kilocode')),
    mcpHint: DEFAULT_MCP_HINT,
  },
  continue: {
    displayName: 'Continue',
    skillsDir: '.continue/skills',
    detect: (root) => fs.existsSync(path.join(root, '.continue')),
    mcpHint: DEFAULT_MCP_HINT,
  },
  goose: {
    displayName: 'Goose',
    skillsDir: '.goose/skills',
    detect: (root) => fs.existsSync(path.join(root, '.goose')),
    mcpHint: DEFAULT_MCP_HINT,
  },
  opencode: {
    displayName: 'OpenCode',
    skillsDir: '.agents/skills',
    detect: (root) => fs.existsSync(path.join(root, '.opencode')),
    mcpHint: DEFAULT_MCP_HINT,
  },
  trae: {
    displayName: 'Trae',
    skillsDir: '.trae/skills',
    detect: (root) => fs.existsSync(path.join(root, '.trae')),
    mcpHint: DEFAULT_MCP_HINT,
  },
  junie: {
    displayName: 'Junie',
    skillsDir: '.junie/skills',
    detect: (root) => fs.existsSync(path.join(root, '.junie')),
    mcpHint: DEFAULT_MCP_HINT,
  },
  'kiro-cli': {
    displayName: 'Kiro CLI',
    skillsDir: '.kiro/skills',
    detect: (root) => fs.existsSync(path.join(root, '.kiro')),
    mcpHint: DEFAULT_MCP_HINT,
  },
  zencoder: {
    displayName: 'Zencoder',
    skillsDir: '.zencoder/skills',
    detect: (root) => fs.existsSync(path.join(root, '.zencoder')),
    mcpHint: DEFAULT_MCP_HINT,
  },
  'qwen-code': {
    displayName: 'Qwen Code',
    skillsDir: '.qwen/skills',
    detect: (root) => fs.existsSync(path.join(root, '.qwen')),
    mcpHint: DEFAULT_MCP_HINT,
  },
};

// --- Registry functions ---

export function getAgentById(id: string): AgentConfig | undefined {
  return agents[id];
}

export function detectAgents(root: string): AgentConfig[] {
  return Object.values(agents).filter((a) => a.detect(root));
}

// --- Backward-compatible exports ---

const AGENT_ALIASES: Record<string, string> = {
  chatgpt: 'chatgpt-codex',
  codex: 'chatgpt-codex',
};

export const AGENTS = [
  { id: 'chatgpt-codex', label: 'ChatGPT (Codex)', targetPath: 'AGENTS.md' },
  { id: 'claude', label: 'Claude Code', targetPath: 'CLAUDE.md' },
  {
    id: 'copilot',
    label: 'GitHub Copilot',
    targetPath: '.github/copilot-instructions.md',
  },
  { id: 'cursor', label: 'Cursor', targetPath: '.cursor/rules/viteplus.mdc' },
  {
    id: 'jetbrains',
    label: 'JetBrains AI Assistant',
    targetPath: '.aiassistant/rules/viteplus.md',
  },
  { id: 'amp', label: 'Amp', targetPath: 'AGENTS.md' },
  { id: 'opencode', label: 'OpenCode', targetPath: 'AGENTS.md' },
  { id: 'other', label: 'Other', targetPath: 'AGENTS.md' },
] as const;

export async function selectAgentTargetPath({
  interactive,
  agent,
  onCancel,
}: {
  interactive: boolean;
  agent?: string | false;
  onCancel: () => void;
}) {
  // Skip entirely if --no-agent is passed
  if (agent === false) {
    return undefined;
  }

  if (interactive && !agent) {
    const selectedAgent = await prompts.select({
      message: 'Which agent are you using?',
      options: [
        {
          label: 'None',
          value: null,
          hint: 'Skip writing agent instructions',
        },
        ...AGENTS.map((option) => ({
          label: option.label,
          value: option.id,
          hint: option.targetPath,
        })),
      ],
      initialValue: 'chatgpt-codex',
    });

    if (prompts.isCancel(selectedAgent)) {
      onCancel();
      return undefined;
    }

    if (selectedAgent === null) {
      return undefined;
    }
    return resolveAgentTargetPath(selectedAgent);
  }

  return resolveAgentTargetPath(agent ?? 'other');
}

export function detectExistingAgentTargetPath(projectRoot: string) {
  for (const option of AGENTS) {
    const targetPath = path.join(projectRoot, option.targetPath);
    if (fs.existsSync(targetPath)) {
      return option.targetPath;
    }
  }
  return undefined;
}

export function resolveAgentTargetPath(agent?: string) {
  if (!agent) {
    return 'AGENTS.md';
  }
  const normalized = normalizeAgentName(agent);
  const alias = AGENT_ALIASES[normalized];
  const resolved = alias ? normalizeAgentName(alias) : normalized;
  const match = AGENTS.find(
    (option) =>
      normalizeAgentName(option.id) === resolved || normalizeAgentName(option.label) === resolved,
  );
  return match?.targetPath ?? 'AGENTS.md';
}

export async function writeAgentInstructions({
  projectRoot,
  targetPath,
  interactive,
}: {
  projectRoot: string;
  targetPath: string | undefined;
  interactive: boolean;
}) {
  if (!targetPath) {
    return;
  }

  const sourcePath = path.join(pkgRoot, 'AGENTS.md');
  if (!fs.existsSync(sourcePath)) {
    prompts.log.warn('Agent instructions template not found; skipping.');
    return;
  }

  const destinationPath = path.join(projectRoot, targetPath);
  await fsPromises.mkdir(path.dirname(destinationPath), { recursive: true });
  if (fs.existsSync(destinationPath)) {
    if (interactive) {
      const action = await prompts.select({
        message: `Agent instructions already exist at ${targetPath}.`,
        options: [
          {
            label: 'Append',
            value: 'append',
            hint: 'Add template content to the end',
          },
          {
            label: 'Skip',
            value: 'skip',
            hint: 'Leave existing file unchanged',
          },
        ],
        initialValue: 'skip',
      });
      if (prompts.isCancel(action) || action === 'skip') {
        prompts.log.info(`Skipped writing ${targetPath}`);
        return;
      }

      const [existingContent, incomingContent] = await Promise.all([
        fsPromises.readFile(destinationPath, 'utf-8'),
        fsPromises.readFile(sourcePath, 'utf-8'),
      ]);
      const separator = existingContent.endsWith('\n') ? '' : '\n';
      await fsPromises.appendFile(destinationPath, `${separator}\n${incomingContent}`);
      prompts.log.success(`Appended agent instructions to ${targetPath}`);
      return;
    }

    prompts.log.info(`Skipped writing ${targetPath} (already exists)`);
    return;
  }

  await fsPromises.copyFile(sourcePath, destinationPath);
  prompts.log.success(`Wrote agent instructions to ${targetPath}`);
}

function normalizeAgentName(value: string) {
  return value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '');
}
