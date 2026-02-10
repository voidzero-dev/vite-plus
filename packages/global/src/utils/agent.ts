import fs from 'node:fs';
import fsPromises from 'node:fs/promises';
import path from 'node:path';

import * as prompts from '@voidzero-dev/vite-plus-prompts';

import { pkgRoot } from './path.js';

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

const AGENT_ALIASES: Record<string, string> = {
  chatgpt: 'chatgpt-codex',
  codex: 'chatgpt-codex',
};
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
