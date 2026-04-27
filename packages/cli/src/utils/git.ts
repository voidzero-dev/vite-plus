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
