import { constants } from 'node:os';

import spawn from 'cross-spawn';

export interface RunCommandOptions {
  command: string;
  args: string[];
  cwd: string;
  envs: NodeJS.ProcessEnv;
  /**
   * Kill the child and reject once exceeded. Without it a wedged child (e.g. a
   * config worker executing a blocking plugin factory) hangs the caller forever.
   */
  timeoutMs?: number;
}

export interface ExecutionResult {
  exitCode: number;
}

export interface RunCommandResult extends ExecutionResult {
  stdout: Buffer;
  stderr: Buffer;
}

function exitCodeFromClose(code: number | null, signal: NodeJS.Signals | null) {
  if (code !== null) {
    return code;
  }
  const signalNumber = signal && constants.signals[signal];
  return signalNumber ? 128 + signalNumber : 1;
}

export async function runCommandSilently(options: RunCommandOptions): Promise<RunCommandResult> {
  const child = spawn(options.command, options.args, {
    // No stdin pipe: leaving one open would deadlock any descendant `.ps1`
    // shim whose `$MyInvocation.ExpectingInput` branch waits for EOF on
    // stdin before invoking `node`.
    stdio: ['ignore', 'pipe', 'pipe'],
    cwd: options.cwd,
    env: options.envs,
    // Own process group (POSIX) so the timeout can kill the whole tree: the
    // child runs arbitrary project code that may spawn its own children.
    detached: process.platform !== 'win32',
  });
  const promise = new Promise<RunCommandResult>((resolve, reject) => {
    const stdout: Buffer[] = [];
    const stderr: Buffer[] = [];
    let timedOut = false;
    // SIGKILL rather than SIGTERM: the timeout exists for children wedged in
    // arbitrary user code, which a catchable signal may never interrupt.
    const timer =
      options.timeoutMs === undefined
        ? undefined
        : setTimeout(() => {
            timedOut = true;
            if (process.platform !== 'win32' && child.pid) {
              try {
                process.kill(-child.pid, 'SIGKILL');
              } catch {
                child.kill('SIGKILL');
              }
            } else {
              child.kill('SIGKILL');
            }
            // A descendant that inherited the pipes can hold them open past
            // the kill (a Windows child tree, or a POSIX process that left
            // the group). Release our ends so `close` always fires; the
            // timeout path rejects without reading the output anyway.
            child.stdout?.destroy();
            child.stderr?.destroy();
          }, options.timeoutMs);
    timer?.unref();
    child.stdout?.on('data', (data) => {
      stdout.push(data);
    });
    child.stderr?.on('data', (data) => {
      stderr.push(data);
    });
    child.on('close', (code, signal) => {
      clearTimeout(timer);
      if (timedOut) {
        reject(new Error(`Command timed out after ${options.timeoutMs}ms: ${options.command}`));
        return;
      }
      resolve({
        exitCode: exitCodeFromClose(code, signal),
        stdout: Buffer.concat(stdout),
        stderr: Buffer.concat(stderr),
      });
    });
    child.on('error', (err) => {
      clearTimeout(timer);
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
    child.on('close', (code, signal) => {
      resolve({ exitCode: exitCodeFromClose(code, signal) });
    });
    child.on('error', (err) => {
      reject(err);
    });
  });
}
