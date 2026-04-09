import fs from 'node:fs';
import fsPromises from 'node:fs/promises';
import path from 'node:path';
import { styleText } from 'node:util';

import * as prompts from '@voidzero-dev/vite-plus-prompts';

import { pkgRoot } from './path.ts';

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

export const AGENTS = [
  {
    id: 'agents',
    label: 'AGENTS.md',
    targetPath: 'AGENTS.md',
    hint: 'Codex, Amp, OpenCode, and similar agents',
    aliases: [
      'agents.md',
      'chatgpt',
      'chatgpt-codex',
      'codex',
      'amp',
      'kilo',
      'kilo-code',
      'kiro',
      'kiro-cli',
      'opencode',
      'other',
    ],
  },
  {
    id: 'claude',
    label: 'CLAUDE.md',
    targetPath: 'CLAUDE.md',
    hint: 'Claude Code',
    aliases: ['claude.md', 'claude-code'],
  },
  {
    id: 'gemini',
    label: 'GEMINI.md',
    targetPath: 'GEMINI.md',
    hint: 'Gemini CLI',
    aliases: ['gemini.md', 'gemini-cli'],
  },
  {
    id: 'copilot',
    label: '.github/copilot-instructions.md',
    targetPath: '.github/copilot-instructions.md',
    hint: 'GitHub Copilot',
    aliases: ['github-copilot', 'copilot-instructions.md'],
  },
  {
    id: 'cursor',
    label: '.cursor/rules/viteplus.mdc',
    targetPath: '.cursor/rules/viteplus.mdc',
    hint: 'Cursor',
    aliases: ['viteplus.mdc'],
  },
  {
    id: 'jetbrains',
    label: '.aiassistant/rules/viteplus.md',
    targetPath: '.aiassistant/rules/viteplus.md',
    hint: 'JetBrains AI Assistant',
    aliases: ['jetbrains', 'jetbrains-ai-assistant', 'aiassistant', 'viteplus.md'],
  },
] as const;

type AgentSelection = string | string[] | false;
const AGENT_DEFAULT_ID = 'agents';
const AGENT_STANDARD_PATH = 'AGENTS.md';
const AGENT_INSTRUCTIONS_START_MARKER = '<!--VITE PLUS START-->';
const AGENT_INSTRUCTIONS_END_MARKER = '<!--VITE PLUS END-->';

const AGENT_ALIASES = Object.fromEntries(
  AGENTS.flatMap((option) =>
    (option.aliases ?? []).map((alias) => [normalizeAgentName(alias), option.id]),
  ),
) as Record<string, string>;

export async function selectAgentTargetPaths({
  interactive,
  agent,
  onCancel,
}: {
  interactive: boolean;
  agent?: AgentSelection;
  onCancel: () => void;
}) {
  // Skip entirely if --no-agent is passed
  if (agent === false) {
    return undefined;
  }

  if (interactive && !agent) {
    const selectedAgents = await prompts.multiselect({
      message: 'Which coding agent instruction files should Vite+ create?',
      options: AGENTS.map((option) => ({
        label: option.label,
        value: option.id,
        hint: option.hint,
      })),
      initialValues: [AGENT_DEFAULT_ID],
      required: false,
    });

    if (prompts.isCancel(selectedAgents)) {
      onCancel();
      return undefined;
    }

    if (selectedAgents.length === 0) {
      return undefined;
    }
    return resolveAgentTargetPaths(selectedAgents);
  }

  return resolveAgentTargetPaths(agent ?? AGENT_DEFAULT_ID);
}

export async function selectAgentTargetPath({
  interactive,
  agent,
  onCancel,
}: {
  interactive: boolean;
  agent?: AgentSelection;
  onCancel: () => void;
}) {
  const targetPaths = await selectAgentTargetPaths({ interactive, agent, onCancel });
  return targetPaths?.[0];
}

export function detectExistingAgentTargetPaths(projectRoot: string) {
  const detectedPaths: string[] = [];
  const seenTargetPaths = new Set<string>();
  for (const option of AGENTS) {
    if (seenTargetPaths.has(option.targetPath)) {
      continue;
    }
    seenTargetPaths.add(option.targetPath);
    const targetPath = path.join(projectRoot, option.targetPath);
    if (fs.existsSync(targetPath) && !fs.lstatSync(targetPath).isSymbolicLink()) {
      detectedPaths.push(option.targetPath);
    }
  }
  return detectedPaths.length > 0 ? detectedPaths : undefined;
}

export function detectExistingAgentTargetPath(projectRoot: string) {
  return detectExistingAgentTargetPaths(projectRoot)?.[0];
}

export function hasExistingAgentInstructions(projectRoot: string): boolean {
  const targetPaths = detectExistingAgentTargetPaths(projectRoot);
  if (!targetPaths) {
    return false;
  }
  for (const targetPath of targetPaths) {
    const content = fs.readFileSync(path.join(projectRoot, targetPath), 'utf-8');
    if (content.includes(AGENT_INSTRUCTIONS_START_MARKER)) {
      return true;
    }
  }
  return false;
}

/**
 * Silently update agent instruction files that contain Vite+ markers.
 * - No agent files → no writes
 * - No Vite+ markers → no writes
 * - Markers present, content up to date → no writes
 * - Markers present, content outdated → update marked section
 */
export function updateExistingAgentInstructions(projectRoot: string): void {
  const targetPaths = detectExistingAgentTargetPaths(projectRoot);
  if (!targetPaths) {
    return;
  }

  const templatePath = path.join(pkgRoot, 'AGENTS.md');
  if (!fs.existsSync(templatePath)) {
    return;
  }

  const templateContent = fs.readFileSync(templatePath, 'utf-8');

  for (const targetPath of targetPaths) {
    try {
      const fullPath = path.join(projectRoot, targetPath);
      const existing = fs.readFileSync(fullPath, 'utf-8');
      const updated = replaceMarkedAgentInstructionsSection(existing, templateContent);
      if (updated !== undefined && updated !== existing) {
        fs.writeFileSync(fullPath, updated);
      }
    } catch {
      // Best-effort: skip files that can't be read or written
    }
  }
}

export function resolveAgentTargetPaths(agent?: string | string[]) {
  const agentNames = parseAgentNames(agent);
  const resolvedAgentNames = agentNames.length > 0 ? agentNames : ['other'];
  const dedupedTargetPaths: string[] = [];
  const seenTargetPaths = new Set<string>();
  for (const name of resolvedAgentNames) {
    const targetPath = resolveSingleAgentTargetPath(name);
    if (seenTargetPaths.has(targetPath)) {
      continue;
    }
    seenTargetPaths.add(targetPath);
    dedupedTargetPaths.push(targetPath);
  }
  return dedupedTargetPaths;
}

export function resolveAgentTargetPath(agent?: string) {
  return resolveAgentTargetPaths(agent)[0] ?? 'AGENTS.md';
}

function parseAgentNames(agent?: string | string[]) {
  if (!agent) {
    return [];
  }
  const values = Array.isArray(agent) ? agent : [agent];
  return values
    .filter((value): value is string => typeof value === 'string')
    .flatMap((value) => value.split(','))
    .map((value) => value.trim())
    .filter((value) => value.length > 0);
}

function resolveSingleAgentTargetPath(agent: string) {
  const normalized = normalizeAgentName(agent);
  const alias = AGENT_ALIASES[normalized];
  const resolved = alias ? normalizeAgentName(alias) : normalized;
  const match = AGENTS.find(
    (option) =>
      normalizeAgentName(option.id) === resolved ||
      normalizeAgentName(option.label) === resolved ||
      normalizeAgentName(option.targetPath) === resolved ||
      option.aliases?.some((candidate) => normalizeAgentName(candidate) === resolved),
  );
  return match?.targetPath ?? AGENT_STANDARD_PATH;
}

export interface AgentConflictInfo {
  targetPath: string;
}

/**
 * Detect agent instruction files that would conflict (exist without markers).
 * Returns only files that need a user decision (append or skip).
 * Read-only — does not write or modify any files.
 */
export async function detectAgentConflicts({
  projectRoot,
  targetPaths,
}: {
  projectRoot: string;
  targetPaths?: string[];
}): Promise<AgentConflictInfo[]> {
  if (!targetPaths || targetPaths.length === 0) {
    return [];
  }

  const sourcePath = path.join(pkgRoot, 'AGENTS.md');
  if (!fs.existsSync(sourcePath)) {
    return [];
  }

  const incomingContent = await fsPromises.readFile(sourcePath, 'utf-8');
  const shouldLinkToAgents = targetPaths.includes(AGENT_STANDARD_PATH);
  const orderedPaths = shouldLinkToAgents
    ? [AGENT_STANDARD_PATH, ...targetPaths.filter((p) => p !== AGENT_STANDARD_PATH)]
    : targetPaths;

  const conflicts: AgentConflictInfo[] = [];
  const seenDestinationPaths = new Set<string>();
  const seenRealPaths = new Set<string>();

  for (const targetPathToCheck of orderedPaths) {
    const destinationPath = path.join(projectRoot, targetPathToCheck);
    const destinationKey = path.resolve(destinationPath);
    if (seenDestinationPaths.has(destinationKey)) {
      continue;
    }
    seenDestinationPaths.add(destinationKey);

    // If linking to AGENTS.md, non-AGENTS.md paths that are not regular files get linked
    if (shouldLinkToAgents && targetPathToCheck !== AGENT_STANDARD_PATH) {
      const existing = await getExistingPathKind(destinationPath);
      if (existing !== 'file') {
        continue;
      }
    }

    if (fs.existsSync(destinationPath)) {
      if (fs.lstatSync(destinationPath).isSymbolicLink()) {
        continue;
      }

      const destinationRealPath = await fsPromises.realpath(destinationPath);
      if (seenRealPaths.has(destinationRealPath)) {
        continue;
      }

      const existingContent = await fsPromises.readFile(destinationPath, 'utf-8');
      const updatedContent = replaceMarkedAgentInstructionsSection(
        existingContent,
        incomingContent,
      );
      if (updatedContent !== undefined) {
        // Has markers — will auto-update, no conflict
        seenRealPaths.add(destinationRealPath);
        continue;
      }

      // Conflict — needs user decision
      conflicts.push({ targetPath: targetPathToCheck });
      seenRealPaths.add(destinationRealPath);
    }
  }

  return conflicts;
}

export async function writeAgentInstructions({
  projectRoot,
  targetPath,
  targetPaths,
  interactive,
  conflictDecisions,
  silent = false,
}: {
  projectRoot: string;
  targetPath?: string;
  targetPaths?: string[];
  interactive: boolean;
  conflictDecisions?: Map<string, 'append' | 'skip'>;
  silent?: boolean;
}) {
  const paths = [...(targetPaths ?? []), ...(targetPath ? [targetPath] : [])];
  if (paths.length === 0) {
    return;
  }

  const sourcePath = path.join(pkgRoot, 'AGENTS.md');
  if (!fs.existsSync(sourcePath)) {
    if (!silent) {
      prompts.log.warn('Agent instructions template not found; skipping.');
    }
    return;
  }

  const seenDestinationPaths = new Set<string>();
  const seenRealPaths = new Set<string>();
  const incomingContent = await fsPromises.readFile(sourcePath, 'utf-8');
  const shouldLinkToAgents = paths.includes(AGENT_STANDARD_PATH);
  const orderedPaths = shouldLinkToAgents
    ? [AGENT_STANDARD_PATH, ...paths.filter((p) => p !== AGENT_STANDARD_PATH)]
    : paths;

  for (const targetPathToWrite of orderedPaths) {
    const destinationPath = path.join(projectRoot, targetPathToWrite);
    const destinationKey = path.resolve(destinationPath);
    if (seenDestinationPaths.has(destinationKey)) {
      continue;
    }
    seenDestinationPaths.add(destinationKey);

    await fsPromises.mkdir(path.dirname(destinationPath), { recursive: true });

    if (shouldLinkToAgents && targetPathToWrite !== AGENT_STANDARD_PATH) {
      const linked = await tryLinkTargetToAgents(projectRoot, targetPathToWrite, silent);
      if (linked) {
        continue;
      }
    }

    if (fs.existsSync(destinationPath)) {
      if (fs.lstatSync(destinationPath).isSymbolicLink()) {
        if (!silent) {
          prompts.log.info(`Skipped writing ${targetPathToWrite} (symlink)`);
        }
        continue;
      }

      const destinationRealPath = await fsPromises.realpath(destinationPath);
      if (seenRealPaths.has(destinationRealPath)) {
        if (!silent) {
          prompts.log.info(`Skipped writing ${targetPathToWrite} (duplicate target)`);
        }
        continue;
      }

      const existingContent = await fsPromises.readFile(destinationPath, 'utf-8');
      const updatedContent = replaceMarkedAgentInstructionsSection(
        existingContent,
        incomingContent,
      );
      if (updatedContent !== undefined) {
        if (updatedContent !== existingContent) {
          await fsPromises.writeFile(destinationPath, updatedContent);
        }
        seenRealPaths.add(destinationRealPath);
        continue;
      }

      // Determine conflict action from pre-resolved decisions, interactive prompt, or default
      let conflictAction: 'append' | 'skip';
      const preResolved = conflictDecisions?.get(targetPathToWrite);
      if (preResolved) {
        conflictAction = preResolved;
      } else if (interactive) {
        const action = await prompts.select({
          message:
            `Agent instructions already exist at ${targetPathToWrite}.\n  ` +
            styleText(
              'gray',
              'The Vite+ template includes guidance on `vp` commands, the build pipeline, and project conventions.',
            ),
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
        conflictAction = prompts.isCancel(action) || action === 'skip' ? 'skip' : 'append';
      } else {
        conflictAction = 'skip';
      }

      if (conflictAction === 'append') {
        await appendAgentContent(
          destinationPath,
          targetPathToWrite,
          existingContent,
          incomingContent,
          silent,
        );
      } else {
        const suffix = !preResolved && !interactive ? ' (already exists)' : '';
        if (!silent) {
          prompts.log.info(`Skipped writing ${targetPathToWrite}${suffix}`);
        }
      }
      seenRealPaths.add(destinationRealPath);
      continue;
    }

    await fsPromises.writeFile(destinationPath, incomingContent);
    if (!silent) {
      prompts.log.success(`Wrote agent instructions to ${targetPathToWrite}`);
    }
    seenRealPaths.add(await fsPromises.realpath(destinationPath));
  }
}

async function appendAgentContent(
  destinationPath: string,
  targetPath: string,
  existingContent: string,
  incomingContent: string,
  silent = false,
) {
  const separator = existingContent.endsWith('\n') ? '' : '\n';
  await fsPromises.appendFile(destinationPath, `${separator}\n${incomingContent}`);
  if (!silent) {
    prompts.log.success(`Appended agent instructions to ${targetPath}`);
  }
}

function normalizeAgentName(value: string) {
  return value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '');
}

export function replaceMarkedAgentInstructionsSection(existing: string, incoming: string) {
  const existingRange = getMarkedRange(
    existing,
    AGENT_INSTRUCTIONS_START_MARKER,
    AGENT_INSTRUCTIONS_END_MARKER,
  );
  if (!existingRange) {
    return undefined;
  }

  const incomingRange = getMarkedRange(
    incoming,
    AGENT_INSTRUCTIONS_START_MARKER,
    AGENT_INSTRUCTIONS_END_MARKER,
  );
  if (!incomingRange) {
    return undefined;
  }

  return `${existing.slice(0, existingRange.start)}${incoming.slice(
    incomingRange.start,
    incomingRange.end,
  )}${existing.slice(existingRange.end)}`;
}

async function tryLinkTargetToAgents(projectRoot: string, targetPath: string, silent = false) {
  const destinationPath = path.join(projectRoot, targetPath);
  const agentsPath = path.join(projectRoot, AGENT_STANDARD_PATH);
  const symlinkTarget = path.relative(path.dirname(destinationPath), agentsPath);
  const existing = await getExistingPathKind(destinationPath);

  if (existing === 'file') {
    return false;
  }

  if (existing === 'symlink') {
    const currentLink = await fsPromises.readlink(destinationPath);
    const resolvedCurrentLink = path.resolve(path.dirname(destinationPath), currentLink);
    if (resolvedCurrentLink === agentsPath) {
      if (!silent) {
        prompts.log.info(
          `Skipped linking ${targetPath} (already linked to ${AGENT_STANDARD_PATH})`,
        );
      }
      return true;
    }
    await fsPromises.unlink(destinationPath);
  }

  try {
    await fsPromises.symlink(symlinkTarget, destinationPath);
  } catch (err: unknown) {
    if ((err as NodeJS.ErrnoException).code === 'EPERM') {
      // On Windows, symlinks require admin privileges.
      // Fall back to copying the file instead.
      await fsPromises.copyFile(agentsPath, destinationPath);
      if (!silent) {
        prompts.log.success(`Copied ${AGENT_STANDARD_PATH} to ${targetPath}`);
      }
      return true;
    }
    throw err;
  }
  if (!silent) {
    prompts.log.success(`Linked ${targetPath} to ${AGENT_STANDARD_PATH}`);
  }
  return true;
}

async function getExistingPathKind(filePath: string) {
  if (!fs.existsSync(filePath)) {
    return 'missing' as const;
  }
  const stat = await fsPromises.lstat(filePath);
  return stat.isSymbolicLink() ? ('symlink' as const) : ('file' as const);
}

function getMarkedRange(content: string, startMarker: string, endMarker: string) {
  const start = content.indexOf(startMarker);
  if (start === -1) {
    return undefined;
  }
  const endMarkerIndex = content.indexOf(endMarker, start + startMarker.length);
  if (endMarkerIndex === -1) {
    return undefined;
  }
  return {
    start,
    end: endMarkerIndex + endMarker.length,
  };
}
