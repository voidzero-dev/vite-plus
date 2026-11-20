import type { ChildProcess } from 'node:child_process';
import { constants } from 'node:os';
import { join } from 'node:path';
import { createInterface } from 'node:readline';
import { fileURLToPath } from 'node:url';

import spawn from 'cross-spawn';
import { onExit } from 'signal-exit';

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
  /** Stop the command tree when neither output stream makes progress. */
  idleTimeoutMs?: number;
  /** When set, arm the idle timer only when this stdout line handler returns true. */
  shouldWatchIdle?: (line: string) => boolean;
}

export interface ExecutionResult {
  exitCode: number;
}

export interface RunCommandResult extends ExecutionResult {
  stdout: Buffer;
  stderr: Buffer;
  idleTimedOut?: true;
}

/** Run this CLI version even when a package-manager launcher does not expose `vp` on PATH. */
export function getVpCommand(
  args: string[],
  command = process.env.VP_CLI_BIN,
): Pick<RunCommandOptions, 'command' | 'args'> {
  if (command !== undefined) {
    return { command, args };
  }
  return getLocalVpCommand(args);
}

/** Run this `vite-plus` package with the current Node.js, even when an older global `vp` launched it. */
export function getLocalVpCommand(args: string[]): Pick<RunCommandOptions, 'command' | 'args'> {
  return {
    command: process.execPath,
    args: [fileURLToPath(import.meta.resolve('vite-plus/bin')), ...args],
  };
}

export function exitCodeFromClose(code: number | null, signal: NodeJS.Signals | null) {
  if (code !== null) {
    return code;
  }
  const signalNumber = signal && constants.signals[signal];
  return signalNumber ? 128 + signalNumber : 1;
}

function stopCommandTree(child: ChildProcess) {
  const pid = child.pid;
  if (pid === undefined) {
    return;
  }
  if (process.platform === 'win32') {
    const taskkill = join(process.env.SystemRoot ?? 'C:\\Windows', 'System32', 'taskkill.exe');
    const result = spawn.sync(taskkill, ['/pid', String(pid), '/t', '/f'], {
      stdio: 'ignore',
      timeout: 5000,
      windowsHide: true,
    });
    if (result.status === 0) {
      return;
    }
  } else {
    // The process group belongs to the caller. Only stop this child's tree.
    const { stdout } = spawn.sync('ps', ['-A', '-o', 'pid=', '-o', 'ppid='], {
      env: { PATH: '/usr/bin:/bin' },
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'ignore'],
      timeout: 5000,
    });
    const childrenByParent = new Map<number, number[]>();
    for (const line of (stdout ?? '').split('\n')) {
      const [childPid, parentPid] = line.trim().split(/\s+/).map(Number);
      if (!Number.isInteger(childPid) || childPid <= 1 || !Number.isInteger(parentPid)) {
        continue;
      }
      const children = childrenByParent.get(parentPid) ?? [];
      children.push(childPid);
      childrenByParent.set(parentPid, children);
    }
    const tree = [pid];
    for (const parent of tree) {
      tree.push(...(childrenByParent.get(parent) ?? []));
    }
    for (let index = tree.length - 1; index > 0; index--) {
      try {
        process.kill(tree[index], 'SIGKILL');
      } catch {
        // A descendant can exit while we walk the tree.
      }
    }
  }
  child.kill('SIGKILL');
}

export async function runCommandSilently(options: RunCommandOptions): Promise<RunCommandResult> {
  const { idleTimeoutMs } = options;
  const timeoutMs = idleTimeoutMs ?? options.timeoutMs;
  const shouldWatchIdle = idleTimeoutMs === undefined ? undefined : options.shouldWatchIdle;
  // Keep the caller's process group for terminal and CI cancellation.
  const child = spawn(options.command, options.args, {
    // No stdin pipe: leaving one open would deadlock any descendant `.ps1`
    // shim whose `$MyInvocation.ExpectingInput` branch waits for EOF on
    // stdin before invoking `node`.
    stdio: ['ignore', 'pipe', 'pipe'],
    cwd: options.cwd,
    env: options.envs,
  });
  const removeExitHandler =
    idleTimeoutMs === undefined
      ? undefined
      : onExit((_code, signal) => {
          // Terminal signals already reach the child in the caller's group.
          if (signal === null) {
            stopCommandTree(child);
          }
        });
  const promise = new Promise<RunCommandResult>((resolve, reject) => {
    const stdout: Buffer[] = [];
    const stderr: Buffer[] = [];
    let timedOut = false;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const stopTimer = () => {
      clearTimeout(timer);
      timer = undefined;
    };
    const startTimer = () => {
      timer = setTimeout(() => {
        timedOut = true;
        if (idleTimeoutMs === undefined) {
          child.kill('SIGKILL');
        } else {
          stopCommandTree(child);
          // A descendant may still hold a pipe open.
          child.stdout?.destroy();
          child.stderr?.destroy();
        }
      }, timeoutMs);
      timer.unref();
    };
    if (timeoutMs !== undefined && !shouldWatchIdle) {
      startTimer();
    }
    const capture = (chunks: Buffer[]) => (data: Buffer) => {
      chunks.push(data);
      if (idleTimeoutMs !== undefined && !timedOut) {
        timer?.refresh();
      }
    };
    child.stdout?.on('data', capture(stdout));
    child.stderr?.on('data', capture(stderr));
    const lines =
      shouldWatchIdle && child.stdout
        ? createInterface({ input: child.stdout, crlfDelay: Infinity })
        : undefined;
    lines?.on('line', (line: string) => {
      if (timedOut) {
        return;
      }
      if (!shouldWatchIdle?.(line)) {
        stopTimer();
      } else if (timer === undefined) {
        startTimer();
      }
    });
    const cleanup = () => {
      lines?.close();
      stopTimer();
      removeExitHandler?.();
    };
    child.on('close', (code, signal) => {
      cleanup();
      if (timedOut && idleTimeoutMs === undefined) {
        reject(new Error(`Command timed out after ${options.timeoutMs}ms: ${options.command}`));
        return;
      }
      resolve({
        ...(timedOut && { idleTimedOut: true as const }),
        exitCode: timedOut ? 124 : exitCodeFromClose(code, signal),
        stdout: Buffer.concat(stdout),
        stderr: Buffer.concat(stderr),
      });
    });
    child.on('error', (err) => {
      cleanup();
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
