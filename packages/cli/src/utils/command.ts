import spawn from 'cross-spawn';

export interface RunCommandOptions {
  command: string;
  args: string[];
  cwd: string;
  envs: NodeJS.ProcessEnv;
}

export interface ExecutionResult {
  exitCode: number;
}

export interface RunCommandResult extends ExecutionResult {
  stdout: Buffer;
  stderr: Buffer;
}

export async function runCommandSilently(options: RunCommandOptions): Promise<RunCommandResult> {
  const child = spawn(options.command, options.args, {
    // No stdin pipe: leaving one open would deadlock any descendant `.ps1`
    // shim whose `$MyInvocation.ExpectingInput` branch waits for EOF on
    // stdin before invoking `node`.
    stdio: ['ignore', 'pipe', 'pipe'],
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
  return new Promise<ExecutionResult>((resolve, reject) => {
    child.on('close', (code) => {
      resolve({ exitCode: code ?? 0 });
    });
    child.on('error', (err) => {
      reject(err);
    });
  });
}
