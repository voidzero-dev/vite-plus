import fs from 'node:fs/promises';
import path from 'node:path';

import spawn from 'cross-spawn';

import { runCommand as runCommandWithFspy } from '../../binding/index.js';
import type { WorkspaceInfo } from '../types/index.js';

export interface ExecutionResult {
  exitCode: number;
  projectDir?: string;
}

export interface RunCommandOptions {
  command: string;
  args: string[];
  cwd: string;
  envs: NodeJS.ProcessEnv;
}

// Run a command and detect the project directory
export async function runCommandAndDetectProjectDir(
  options: RunCommandOptions,
  parentDir?: string,
): Promise<ExecutionResult> {
  const cwd = parentDir ? path.join(options.cwd, parentDir) : options.cwd;
  const existingDirs = new Set<string>();
  if (parentDir) {
    await fs.mkdir(cwd, { recursive: true });
    // Get existing subdirectories before running the command
    const entries = await fs.readdir(cwd, { withFileTypes: true });
    for (const entry of entries) {
      if (entry.isDirectory()) {
        existingDirs.add(entry.name);
      }
    }
  }

  const result = await runCommandWithFspy({
    binName: options.command,
    args: options.args,
    envs: options.envs as Record<string, string>,
    cwd,
  });

  // Detect project directory from path accesses
  // Find the closest directory containing package.json relative to cwd
  let projectDir: string | undefined;
  let minDepth = Infinity;

  for (const [filePath, pathAccess] of Object.entries(result.pathAccesses)) {
    // Look for package.json writes
    if (
      pathAccess.write &&
      filePath.endsWith('package.json') &&
      !filePath.includes('node_modules')
    ) {
      // Extract directory from package.json path
      const dir = path.dirname(filePath);

      // Skip if it's the current directory
      if (dir === '.' || dir === '') {
        continue;
      }
      // Skip if this is an existing directory (created before the command ran)
      if (existingDirs.has(dir)) {
        continue;
      }

      // Calculate depth (number of path segments)
      const depth = dir.split(path.sep).length;

      // Keep the closest (shallowest) directory
      if (depth < minDepth) {
        minDepth = depth;
        projectDir = dir;
      }
    }
  }

  // If parentDir is provided, join it with the project directory
  if (parentDir && projectDir) {
    projectDir = path.join(parentDir, projectDir);
  }

  return {
    exitCode: result.exitCode,
    projectDir,
  };
}

export interface RunCommandResult extends ExecutionResult {
  stdout: Buffer;
  stderr: Buffer;
}

export async function runCommandSilently(options: RunCommandOptions): Promise<RunCommandResult> {
  const child = spawn(options.command, options.args, {
    stdio: 'pipe',
    cwd: options.cwd,
    env: options.envs,
  });
  const promise = new Promise<RunCommandResult>((resolve, reject) => {
    const stdout: Buffer[] = [];
    const stderr: Buffer[] = [];
    child.stdout?.on('data', (data) => {
      stdout.push(data);
    });
    child.stderr?.on('data', (data) => {
      stderr.push(data);
    });
    child.on('close', (code) => {
      resolve({
        exitCode: code ?? 0,
        stdout: Buffer.concat(stdout),
        stderr: Buffer.concat(stderr),
      });
    });
    child.on('error', (err) => {
      reject(err);
    });
  });
  return await promise;
}

export async function runCommand(options: RunCommandOptions): Promise<ExecutionResult> {
  const child = spawn(options.command, options.args, {
    stdio: 'inherit',
    cwd: options.cwd,
    env: options.envs,
  });
  const promise = new Promise<ExecutionResult>((resolve, reject) => {
    child.on('close', (code) => {
      resolve({ exitCode: code ?? 0 });
    });
    child.on('error', (err) => {
      reject(err);
    });
  });
  return await promise;
}

// Get the package runner command for each package manager
export function getPackageRunner(workspaceInfo: WorkspaceInfo) {
  switch (workspaceInfo.packageManager) {
    case 'pnpm':
      return {
        command: 'pnpm',
        args: ['dlx'],
      };
    case 'yarn':
      return {
        command: 'yarn',
        args: ['dlx'],
      };
    case 'bun':
      return { command: 'bun', args: ['x'] };
    case 'npm':
    default:
      return { command: 'npx', args: [] };
  }
}

// TODO: will use `vp dlx` instead, see https://github.com/voidzero-dev/vite-task/issues/27
export function formatDlxCommand(
  packageName: string,
  args: string[],
  workspaceInfo: WorkspaceInfo,
) {
  const runner = getPackageRunner(workspaceInfo);
  const dlxArgs = runner.command === 'npx' ? ['--', ...args] : args;
  return {
    command: runner.command,
    args: [...runner.args, packageName, ...dlxArgs],
  };
}

export function prependToPathToEnvs(extraPath: string, envs: NodeJS.ProcessEnv) {
  const delimiter = path.delimiter;
  const pathKey = Object.keys(envs).find((key) => key.toLowerCase() === 'path') ?? 'PATH';

  const current = envs[pathKey] ?? '';

  // avoid duplicate
  const parts = current.split(delimiter).filter(Boolean);
  if (!parts.includes(extraPath)) {
    envs[pathKey] = extraPath + (current ? delimiter + current : '');
  }
  return envs;
}
