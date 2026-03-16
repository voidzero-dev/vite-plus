import {
  existsSync,
  lstatSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  readlinkSync,
  realpathSync,
  symlinkSync,
} from 'node:fs';
import { join, relative } from 'node:path';

import * as prompts from '@voidzero-dev/vite-plus-prompts';

import { type AgentConfig } from './agent.js';
import { VITE_PLUS_NAME } from './constants.js';
import { pkgRoot } from './path.js';

interface SkillInfo {
  dirName: string;
  name: string;
  description: string;
}

export function parseSkills(skillsDir: string): SkillInfo[] {
  if (!existsSync(skillsDir)) {
    return [];
  }
  const entries = readdirSync(skillsDir, { withFileTypes: true });
  const skills: SkillInfo[] = [];
  for (const entry of entries) {
    if (!entry.isDirectory()) {
      continue;
    }
    const skillMd = join(skillsDir, entry.name, 'SKILL.md');
    if (!existsSync(skillMd)) {
      continue;
    }
    const content = readFileSync(skillMd, 'utf-8');
    const frontmatter = content.match(/^---\n([\s\S]*?)\n---/);
    if (!frontmatter) {
      prompts.log.warn(`  Skipping ${entry.name}: SKILL.md is missing valid frontmatter`);
      continue;
    }
    const nameMatch = frontmatter[1].match(/^name:\s*(.+)$/m);
    const descMatch = frontmatter[1].match(/^description:\s*(.+)$/m);
    skills.push({
      dirName: entry.name,
      name: nameMatch ? nameMatch[1].trim() : entry.name,
      description: descMatch ? descMatch[1].trim() : '',
    });
  }
  return skills;
}

/** Check if a path exists on disk, including broken symlinks that existsSync misses. */
function pathExists(p: string): boolean {
  try {
    lstatSync(p);
    return true;
  } catch {
    return false;
  }
}

function linkSkills(
  root: string,
  skillsDir: string,
  skills: SkillInfo[],
  agentSkillsDir: string,
): number {
  const targetDir = join(root, agentSkillsDir);
  if (!existsSync(targetDir)) {
    mkdirSync(targetDir, { recursive: true });
  }

  const isWindows = process.platform === 'win32';
  const symlinkType = isWindows ? 'junction' : 'dir';

  let linked = 0;
  for (const skill of skills) {
    const linkPath = join(targetDir, skill.dirName);
    const sourcePath = join(skillsDir, skill.dirName);
    const relativeTarget = relative(targetDir, sourcePath);
    const symlinkTarget = isWindows ? sourcePath : relativeTarget;

    if (pathExists(linkPath)) {
      try {
        const existing = readlinkSync(linkPath);
        if (existing === symlinkTarget) {
          prompts.log.info(`  ${skill.name} — already linked`);
          continue;
        }
      } catch (err: unknown) {
        if ((err as NodeJS.ErrnoException).code !== 'EINVAL') {
          prompts.log.warn(
            `  ${skill.name} — failed to read existing path: ${(err as Error).message}`,
          );
          continue;
        }
      }
      prompts.log.warn(`  ${skill.name} — path exists but is not the expected symlink, skipping`);
      continue;
    }

    try {
      symlinkSync(symlinkTarget, linkPath, symlinkType);
    } catch (err: unknown) {
      prompts.log.warn(`  ${skill.name} — failed to create symlink: ${(err as Error).message}`);
      continue;
    }
    prompts.log.success(`  ${skill.name} — linked`);
    linked++;
  }
  return linked;
}

function getStableSkillsDir(root: string): string {
  const resolvedSkillsDir = join(pkgRoot, 'skills');
  // Prefer the logical node_modules path for a cleaner, stable symlink
  // (avoids pnpm's versioned .pnpm/pkg@version/... real path)
  const logicalSkillsDir = join(root, 'node_modules', VITE_PLUS_NAME, 'skills');
  try {
    if (realpathSync(logicalSkillsDir) === realpathSync(resolvedSkillsDir)) {
      return logicalSkillsDir;
    }
  } catch {
    // Fall through to resolved path
  }
  return resolvedSkillsDir;
}

export function linkSkillsForSpecificAgents(root: string, agentConfigs: AgentConfig[]): number {
  const skillsDir = getStableSkillsDir(root);
  const skills = parseSkills(skillsDir);
  if (skills.length === 0) {
    return 0;
  }

  if (agentConfigs.length === 0) {
    return 0;
  }

  let totalLinked = 0;
  for (const agent of agentConfigs) {
    prompts.log.info(`${agent.displayName} → ${agent.skillsDir}`);
    totalLinked += linkSkills(root, skillsDir, skills, agent.skillsDir);
  }
  return totalLinked;
}
