import { execFile, type ChildProcess } from 'node:child_process';
import { readdirSync, readFileSync } from 'node:fs';
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

function listProcfsParents(): [number, number][] | undefined {
  if (process.platform === 'linux') {
    try {
      // Minimal Linux images may not ship ps.
      return readdirSync('/proc')
        .filter((entry) => /^\d+$/.test(entry))
        .flatMap((pid): [number, number][] => {
          try {
            const status = readFileSync(`/proc/${pid}/status`, 'utf8');
            const parent = /^PPid:[ \t]+(\d+)$/m.exec(status);
            return parent ? [[Number(pid), Number(parent[1])]] : [];
          } catch {
            // An exited or inaccessible process has no readable status.
            return [];
          }
        });
    } catch {
      // Fall back to ps if procfs is unavailable.
    }
  }
  return undefined;
}

function stopPosixCommandTree(child: ChildProcess, pid: number, wait: boolean) {
  const stopTree = (parents: [number, number][]) => {
    // The process group belongs to the caller. Only stop this child's tree.
    if (child.exitCode !== null || child.signalCode !== null) {
      return;
    }
    const childrenByParent = new Map<number, number[]>();
    for (const [childPid, parentPid] of parents) {
      if (!Number.isInteger(childPid) || childPid <= 1 || !Number.isInteger(parentPid)) {
        continue;
      }
      const children = childrenByParent.get(parentPid) ?? [];
      children.push(childPid);
      childrenByParent.set(parentPid, children);
    }
    const tree = new Set([pid]);
    for (const parent of tree) {
      for (const childPid of childrenByParent.get(parent) ?? []) {
        tree.add(childPid);
      }
    }
    tree.delete(pid);
    for (const childPid of Array.from(tree).toReversed()) {
      try {
        process.kill(childPid, 'SIGKILL');
      } catch {
        // A descendant can exit while we walk the tree.
      }
    }
    child.kill('SIGKILL');
  };
  const parents = listProcfsParents();
  if (parents !== undefined) {
    stopTree(parents);
    return;
  }
  const args = ['-A', '-o', 'pid=', '-o', 'ppid='];
  const options = { env: { PATH: '/usr/bin:/bin' }, timeout: 5000 };
  const stopFromPs = (stdout: string) => {
    stopTree(
      stdout.split('\n').map((line) => {
        const [childPid, parentPid] = line.trim().split(/\s+/, 2);
        return [Number(childPid), Number(parentPid)];
      }),
    );
  };
  if (wait) {
    const { stdout } = spawn.sync('ps', args, {
      ...options,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'ignore'],
    });
    stopFromPs(stdout ?? '');
  } else {
    execFile('ps', args, options, (_error, stdout) => stopFromPs(stdout));
  }
}

function stopCommandTree(child: ChildProcess, { wait = true } = {}) {
  const pid = child.pid;
  if (pid === undefined || child.exitCode !== null || child.signalCode !== null) {
    return;
  }
  if (process.platform !== 'win32') {
    stopPosixCommandTree(child, pid, wait);
    return;
  }
  const taskkill = join(process.env.SystemRoot ?? 'C:\\Windows', 'System32', 'taskkill.exe');
  const args = ['/pid', String(pid), '/t', '/f'];
  const options = { timeout: 5000, windowsHide: true };
  if (!wait) {
    execFile(taskkill, args, options, (error) => {
      if (error) {
        child.kill('SIGKILL');
      }
    });
    return;
  }
  const result = spawn.sync(taskkill, args, { ...options, stdio: 'ignore' });
  if (result.status !== 0) {
    child.kill('SIGKILL');
  }
}

export async function runCommandSilently(options: RunCommandOptions): Promise<RunCommandResult> {
  const { idleTimeoutMs } = options;
  const timeoutMs = idleTimeoutMs ?? options.timeoutMs;
  const hasDeadline = idleTimeoutMs === undefined && timeoutMs !== undefined;
  const shouldWatchIdle = idleTimeoutMs === undefined ? undefined : options.shouldWatchIdle;
  // Captured commands stay in the caller's group for terminal and CI cancellation.
  const child = spawn(options.command, options.args, {
    // No stdin pipe: leaving one open would deadlock any descendant `.ps1`
    // shim whose `$MyInvocation.ExpectingInput` branch waits for EOF on
    // stdin before invoking `node`.
    stdio: ['ignore', 'pipe', 'pipe'],
    cwd: options.cwd,
    env: options.envs,
  });
  const removeExitHandler =
    idleTimeoutMs !== undefined
      ? onExit((_code, signal) => {
          if (signal === null) {
            // Terminal signals already reach the child in the caller's group.
            stopCommandTree(child);
          }
        })
      : undefined;
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
        // Deadline callers can proceed while platform cleanup finishes.
        stopCommandTree(child, { wait: !hasDeadline });
        // A descendant may still hold a pipe open after the wrapper exits.
        child.stdout?.destroy();
        child.stderr?.destroy();
        if (hasDeadline) {
          cleanup();
          reject(new Error(`Command timed out after ${timeoutMs}ms: ${options.command}`));
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
      if (timedOut && hasDeadline) {
        return;
      }
      cleanup();
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
