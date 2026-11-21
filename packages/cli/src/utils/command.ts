import { execFile, type ChildProcess } from 'node:child_process';
import {
  closeSync,
  fstatSync,
  mkdtempSync,
  openSync,
  readlinkSync,
  readdirSync,
  readFileSync,
  readSync,
  rmdirSync,
  rmSync,
  unlinkSync,
} from 'node:fs';
import { constants, tmpdir } from 'node:os';
import { basename, join } from 'node:path';
import { createInterface } from 'node:readline';
import { PassThrough } from 'node:stream';
import { fileURLToPath } from 'node:url';

import spawn from 'cross-spawn';
import { onExit } from 'signal-exit';

export const MAX_TIMER_DELAY_MS = 0x7fff_ffff;

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
  /** Arm the idle timer while this handler returns true. It can defer the timer for a known wait. */
  shouldWatchIdle?: (line: string, deferIdleTimeout: (delayMs: number) => void) => boolean;
  /** Let package downloads and Git fetches finish before declaring the install idle. */
  waitForPackageFetch?: boolean;
  /** Capture stdout through a regular file so a child can finish writing before process.exit(). */
  stdoutToFile?: boolean;
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

type ProcessEntry = [pid: number, parentPid: number, executable: string];

const psArgs = ['-A', '-o', 'pid=', '-o', 'ppid='];
const psOptions = { env: { PATH: '/usr/bin:/bin' }, timeout: 5000 };

function getWindowsPowerShellPath() {
  return join(
    process.env.SystemRoot ?? 'C:\\Windows',
    'System32/WindowsPowerShell/v1.0/powershell.exe',
  );
}

function listProcfsProcesses(): ProcessEntry[] | undefined {
  if (process.platform === 'linux') {
    try {
      // Minimal Linux images may not ship ps.
      return readdirSync('/proc')
        .filter((entry) => /^\d+$/.test(entry))
        .flatMap((pid): ProcessEntry[] => {
          try {
            const status = readFileSync(`/proc/${pid}/status`, 'utf8');
            const parent = /^PPid:[ \t]+(\d+)$/m.exec(status);
            const name = /^Name:[ \t]+([^\n]+)/m.exec(status)?.[1] ?? '';
            const executable = /^State:[ \t]+Z\b/m.test(status) ? '' : name;
            return parent ? [[Number(pid), Number(parent[1]), executable]] : [];
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

function parseProcessTable(stdout: string, hasState: boolean): ProcessEntry[] {
  return stdout.split(/\r?\n/).flatMap((line): ProcessEntry[] => {
    const row = /^\s*(\d+)\s+(\d+)(?:\s+(.*))?$/.exec(line);
    if (!row) {
      return [];
    }
    const details = row[3] ?? '';
    const state = hasState ? /^(\S+)\s+/.exec(details) : null;
    const executable = state?.[1].startsWith('Z')
      ? ''
      : details.slice(state?.[0].length ?? 0).trim();
    return [[Number(row[1]), Number(row[2]), executable]];
  });
}

function getCommandDescendants(pid: number, processes: ProcessEntry[]): ProcessEntry[] {
  const childrenByParent = new Map<number, ProcessEntry[]>();
  for (const process of processes) {
    const [childPid, parentPid] = process;
    if (!Number.isInteger(childPid) || childPid <= 1 || !Number.isInteger(parentPid)) {
      continue;
    }
    const children = childrenByParent.get(parentPid) ?? [];
    children.push(process);
    childrenByParent.set(parentPid, children);
  }
  const visited = new Set([pid]);
  const descendants: ProcessEntry[] = [];
  for (const parent of visited) {
    for (const process of childrenByParent.get(parent) ?? []) {
      if (!visited.has(process[0])) {
        visited.add(process[0]);
        descendants.push(process);
      }
    }
  }
  return descendants;
}

function hasProcfsTcpConnection(pids: number[]): boolean {
  const sockets = new Set<string>();
  for (const path of ['/proc/net/tcp', '/proc/net/tcp6']) {
    try {
      for (const line of readFileSync(path, 'utf8').split('\n')) {
        const fields = line.trim().split(/\s+/);
        if (fields[3] === '01' || fields[3] === '02') {
          // ESTABLISHED or SYN_SENT.
          sockets.add(`socket:[${fields[9]}]`);
        }
      }
    } catch {
      // A network namespace may not expose both address families.
    }
  }
  return (
    sockets.size > 0 &&
    pids.some((pid) => {
      const directory = `/proc/${pid}/fd`;
      try {
        return readdirSync(directory).some((fd) => {
          try {
            return sockets.has(readlinkSync(join(directory, fd)));
          } catch {
            // The descriptor may close during the check.
            return false;
          }
        });
      } catch {
        // The process may have exited.
        return false;
      }
    })
  );
}

async function hasTcpConnection(pids: number[], signal: AbortSignal): Promise<boolean> {
  if (process.platform === 'linux') {
    return hasProcfsTcpConnection(pids);
  }
  const windows = process.platform === 'win32';
  const command = windows ? getWindowsPowerShellPath() : '/usr/sbin/lsof';
  // PowerShell filters by TCP state and returns numeric PIDs in every locale.
  const args = windows
    ? [
        '-NoProfile',
        '-NonInteractive',
        '-Command',
        `Get-NetTCPConnection -State Established,SynSent -OwningProcess ${pids.join(',')} ` +
          '-ErrorAction SilentlyContinue | Select-Object -ExpandProperty OwningProcess',
      ]
    : ['-nP', '-a', '-p', pids.join(','), '-iTCP', '-FT'];
  return new Promise<boolean>((resolve) => {
    execFile(command, args, { signal, timeout: 5000, windowsHide: true }, (error, output) => {
      if (!windows) {
        // lsof can report connections and return 1 when another selected PID has exited.
        resolve(/^TST=(?:ESTABLISHED|SYN_SENT)$/m.test(output));
        return;
      }
      if (error) {
        resolve(false);
        return;
      }
      const owners = new Set(pids);
      resolve(output.split(/\r?\n/).some((line) => owners.has(Number(line.trim()))));
    });
  });
}

async function hasActivePackageFetch(pid: number, signal: AbortSignal): Promise<boolean> {
  const windows = process.platform === 'win32';
  let processes = listProcfsProcesses();
  if (processes === undefined) {
    const command = windows ? getWindowsPowerShellPath() : 'ps';
    const args = windows
      ? [
          '-NoProfile',
          '-NonInteractive',
          '-Command',
          'Get-CimInstance Win32_Process -Property ProcessId,ParentProcessId,Name | ' +
            "ForEach-Object { '{0} {1} {2}' -f $_.ProcessId, $_.ParentProcessId, $_.Name }",
        ]
      : [...psArgs, '-o', 'state=', '-o', 'comm='];
    const options = windows ? { timeout: 5000, windowsHide: true } : psOptions;
    const stdout = await new Promise<string>((resolve) => {
      execFile(command, args, { ...options, signal }, (error, output) =>
        resolve(error ? '' : output),
      );
    });
    processes = parseProcessTable(stdout, !windows);
  }
  const git = windows ? 'git.exe' : 'git';
  const descendants = getCommandDescendants(pid, processes);
  return (
    descendants.some(([, , executable]) => basename(executable).toLowerCase() === git) ||
    hasTcpConnection([pid, ...descendants.map(([pid]) => pid)], signal)
  );
}

function stopPosixCommandTree(child: ChildProcess, pid: number, wait: boolean) {
  const stopTree = (processes: ProcessEntry[]) => {
    // The process group belongs to the caller. Only stop this child's tree.
    if (child.exitCode !== null || child.signalCode !== null) {
      return;
    }
    for (const [childPid] of getCommandDescendants(pid, processes).toReversed()) {
      try {
        process.kill(childPid, 'SIGKILL');
      } catch {
        // A descendant can exit while we walk the tree.
      }
    }
    child.kill('SIGKILL');
  };
  const processes = listProcfsProcesses();
  if (processes !== undefined) {
    stopTree(processes);
    return;
  }
  const stopFromPs = (stdout: string) => stopTree(parseProcessTable(stdout, false));
  if (wait) {
    const { stdout } = spawn.sync('ps', psArgs, {
      ...psOptions,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'ignore'],
    });
    stopFromPs(stdout ?? '');
  } else {
    execFile('ps', psArgs, psOptions, (_error, stdout) => stopFromPs(stdout));
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

function openStdoutFile(): number | undefined {
  let directory: string | undefined;
  let fd: number | undefined;
  try {
    directory = mkdtempSync(join(tmpdir(), 'vp-command-'));
    const file = join(directory, 'stdout');
    fd = openSync(file, 'wx+', 0o600);
    // Unlink before launch so cancellation cannot leave captured output on disk.
    unlinkSync(file);
    rmdirSync(directory);
    return fd;
  } catch {
    // Fall back to the pipe if temporary file capture is unavailable.
    if (fd !== undefined) {
      closeSync(fd);
    }
    if (directory !== undefined) {
      rmSync(directory, { recursive: true, force: true, maxRetries: 3 });
    }
    return undefined;
  }
}

export async function runCommandSilently(options: RunCommandOptions): Promise<RunCommandResult> {
  // Node writes to pipes synchronously on Windows.
  const stdoutFd =
    options.stdoutToFile && process.platform !== 'win32' ? openStdoutFile() : undefined;
  try {
    return await runCapturedCommand(options, stdoutFd);
  } finally {
    if (stdoutFd !== undefined) {
      closeSync(stdoutFd);
    }
  }
}

function runCapturedCommand(
  options: RunCommandOptions,
  stdoutFd: number | undefined,
): Promise<RunCommandResult> {
  const { idleTimeoutMs } = options;
  const timeoutMs = idleTimeoutMs ?? options.timeoutMs;
  const hasDeadline = idleTimeoutMs === undefined && timeoutMs !== undefined;
  const shouldWatchIdle = idleTimeoutMs === undefined ? undefined : options.shouldWatchIdle;
  const fetchQuery =
    idleTimeoutMs !== undefined && options.waitForPackageFetch ? new AbortController() : undefined;
  // Captured commands stay in the caller's group for terminal and CI cancellation.
  const child = spawn(options.command, options.args, {
    // No stdin pipe: leaving one open would deadlock any descendant `.ps1`
    // shim whose `$MyInvocation.ExpectingInput` branch waits for EOF on
    // stdin before invoking `node`.
    stdio: ['ignore', stdoutFd ?? 'pipe', 'pipe'],
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
  return new Promise<RunCommandResult>((resolve, reject) => {
    const stdout: Buffer[] = [];
    const stderr: Buffer[] = [];
    const fileOutput = stdoutFd === undefined ? undefined : new PassThrough();
    const stdoutStream = fileOutput ?? child.stdout;
    let offset = 0;
    let settled = false;
    let timedOut = false;
    let watching = false;
    let fetchWasRunning = false;
    let filePoll: ReturnType<typeof setInterval> | undefined;
    let timer: ReturnType<typeof setTimeout> | undefined;
    let deferredUntil = 0;
    let deferTimer: ReturnType<typeof setTimeout> | undefined;
    const stopTimer = () => {
      clearTimeout(timer);
      timer = undefined;
    };
    const expireTimer = async () => {
      // Check for bytes written since the last poll before deciding it is idle.
      if (idleTimeoutMs !== undefined && readFileOutput()) {
        return;
      }
      if (
        fetchQuery &&
        child.pid !== undefined &&
        child.exitCode === null &&
        child.signalCode === null
      ) {
        const timerAtCheck = timer;
        const chunks = stdout.length + stderr.length;
        const fetchIsRunning = await hasActivePackageFetch(child.pid, fetchQuery.signal);
        // Output can arrive or resolution can finish during the process query.
        if (
          settled ||
          timer !== timerAtCheck ||
          stdout.length + stderr.length !== chunks ||
          readFileOutput()
        ) {
          return;
        }
        const allowFetchToFinish = fetchIsRunning || fetchWasRunning;
        fetchWasRunning = fetchIsRunning;
        if (allowFetchToFinish) {
          // pnpm can transfer a package without reporting progress. Let its own
          // network timeouts handle open connections and allow time after a fetch ends.
          startTimer();
          return;
        }
      }
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
    };
    const startTimer = () => {
      timer = setTimeout(() => void expireTimer(), timeoutMs);
      timer.unref();
    };
    const deferIdleTimeout = (delayMs: number) => {
      if (!Number.isFinite(delayMs) || delayMs <= 0 || delayMs > MAX_TIMER_DELAY_MS) {
        return;
      }
      const until = performance.now() + delayMs;
      // Another request can be scheduled later while this one is still waiting.
      if (deferTimer && until <= deferredUntil) {
        return;
      }
      deferredUntil = until;
      stopTimer();
      clearTimeout(deferTimer);
      deferTimer = setTimeout(() => {
        deferTimer = undefined;
        if (watching) {
          startTimer();
        }
      }, delayMs).unref();
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
    stdoutStream?.on('data', capture(stdout));
    child.stderr?.on('data', capture(stderr));
    const lines =
      shouldWatchIdle && stdoutStream
        ? createInterface({ input: stdoutStream, crlfDelay: Infinity })
        : undefined;
    lines?.on('line', (line: string) => {
      if (timedOut) {
        return;
      }
      watching = shouldWatchIdle?.(line, deferIdleTimeout) ?? false;
      if (!watching) {
        fetchWasRunning = false;
        stopTimer();
      } else if (timer === undefined && deferTimer === undefined) {
        startTimer();
      }
    });
    const cleanup = () => {
      settled = true;
      fetchQuery?.abort();
      clearInterval(filePoll);
      lines?.close();
      fileOutput?.destroy();
      stopTimer();
      clearTimeout(deferTimer);
      removeExitHandler?.();
    };
    const readFileOutput = () => {
      if (stdoutFd === undefined || !fileOutput) {
        return false;
      }
      const start = offset;
      try {
        // Drain only the bytes available at the start of this poll.
        const size = fstatSync(stdoutFd).size;
        while (offset < size) {
          const buffer = Buffer.allocUnsafe(Math.min(64 * 1024, size - offset));
          // A positional read leaves the child's write offset alone.
          const bytes = readSync(stdoutFd, buffer, { position: offset });
          if (bytes === 0) {
            break;
          }
          offset += bytes;
          fileOutput.write(buffer.subarray(0, bytes));
        }
      } catch (error) {
        cleanup();
        stopCommandTree(child, { wait: false });
        child.stderr?.destroy();
        reject(error);
        return true;
      }
      return offset > start;
    };
    if (fileOutput) {
      filePoll = setInterval(readFileOutput, 100);
      filePoll.unref();
    }
    child.on('close', (code, signal) => {
      if (settled) {
        return;
      }
      readFileOutput();
      if (settled) {
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
      if (!settled) {
        cleanup();
        reject(err);
      }
    });
  });
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
