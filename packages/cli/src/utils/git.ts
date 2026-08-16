import fs from 'node:fs';
import path from 'node:path';

import * as prompts from '@voidzero-dev/vite-plus-prompts';

import { runCommandSilently } from './command.ts';

/**
 * Walk up from `startPath` looking for `.git` (directory or file — submodules
 * use a `.git` file).  Returns the directory that contains `.git`, or `null`.
 */
export function findGitRoot(startPath: string): string | null {
  let dir = startPath;
  while (true) {
    if (fs.existsSync(path.join(dir, '.git'))) {
      return dir;
    }
    const parent = path.dirname(dir);
    if (parent === dir) {
      return null;
    }
    dir = parent;
  }
}

export async function initGitRepository(cwd: string): Promise<boolean> {
  const result = await runCommandSilently({
    command: 'git',
    args: ['init'],
    cwd,
    envs: process.env,
  });
  if (result.exitCode !== 0) {
    prompts.log.warn('Failed to initialize git repository');
    const stderr = result.stderr.toString().trim();
    if (stderr) {
      prompts.log.info(stderr);
    }
    return false;
  }
  return true;
}
