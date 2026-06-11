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

export interface InitialCommitResult {
  success: boolean;
  /** Combined stdout/stderr from `git commit`, for diagnosing failures (e.g. a failing pre-commit hook). */
  output: string;
}

export async function createInitialCommit(cwd: string): Promise<InitialCommitResult> {
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
  return {
    success: result.exitCode === 0,
    output: `${result.stdout.toString()}${result.stderr.toString()}`.trim(),
  };
}
