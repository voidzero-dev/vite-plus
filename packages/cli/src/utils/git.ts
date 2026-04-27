import { runCommandSilently } from './command.ts';

export async function initGitRepository(cwd: string): Promise<boolean> {
  const result = await runCommandSilently({
    command: 'git',
    args: ['init'],
    cwd,
    envs: process.env,
  });
  return result.exitCode === 0;
}

export async function createInitialCommit(cwd: string): Promise<boolean> {
  await runCommandSilently({
    command: 'git',
    args: ['add', '-A'],
    cwd,
    envs: process.env,
  });
  const result = await runCommandSilently({
    command: 'git',
    args: ['commit', '-m', 'Initial commit from Vite+'],
    cwd,
    envs: process.env,
  });
  return result.exitCode === 0;
}
