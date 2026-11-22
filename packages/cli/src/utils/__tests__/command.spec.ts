import { execFile, execFileSync, spawn } from 'node:child_process';
import { once } from 'node:events';
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { rm } from 'node:fs/promises';
import { createServer } from 'node:net';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { setTimeout } from 'node:timers/promises';

import crossSpawn from 'cross-spawn';
import { describe, expect, it, vi } from 'vitest';

import { runCommand, runCommandSilently } from '../command.ts';

vi.mock('node:child_process', { spy: true });

function stopFixtureChild(pidFile: string) {
  if (existsSync(pidFile)) {
    try {
      process.kill(Number(readFileSync(pidFile, 'utf8')), 'SIGKILL');
    } catch {
      // The command runner may already have stopped the child.
    }
  }
}

function fixtureChildExists(pid: number): boolean {
  try {
    process.kill(pid, 0);
    return true;
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== 'ESRCH') {
      throw error;
    }
    return false;
  }
}

function denyProcessUtility(): never {
  throw Object.assign(new Error('spawn EPERM'), { code: 'EPERM' });
}

function fixtureProcessGroup(pid: string): number {
  if (process.platform === 'linux') {
    const stat = readFileSync(`/proc/${pid}/stat`, 'utf8');
    // Fields after the parenthesized command name start with state, PPID, and PGID.
    return Number(stat.slice(stat.lastIndexOf(')') + 2).split(' ')[2]);
  }
  return Number(execFileSync('ps', ['-o', 'pgid=', '-p', pid], { encoding: 'utf8' }));
}

describe('command runners', () => {
  it('resolves with the child output when no timeout is set', async () => {
    const result = await runCommandSilently({
      command: process.execPath,
      args: ['-e', 'process.stdout.write("ok")'],
      cwd: process.cwd(),
      envs: process.env,
    });
    expect(result.exitCode).toBe(0);
    expect(result.stdout.toString()).toBe('ok');
  });

  it('captures a large stdout write followed immediately by exit', async () => {
    const size = 8 * 1024 * 1024;
    const ending = '\ndependency@999.0.0\n';
    const result = await runCommandSilently({
      command: process.execPath,
      args: [
        '-e',
        `process.stdout.write('x'.repeat(${size}) + ${JSON.stringify(ending)});
        process.exit(7);`,
      ],
      cwd: process.cwd(),
      envs: process.env,
      stdoutToFile: true,
    });
    expect(result.exitCode).toBe(7);
    expect(result.stdout.length).toBe(size + Buffer.byteLength(ending));
    expect(result.stdout.subarray(size).toString()).toBe(ending);
  });

  it('kills a wedged child and rejects once timeoutMs is exceeded', async () => {
    await expect(
      runCommandSilently({
        command: process.execPath,
        // A child that never exits on its own (the blocking-plugin-factory case).
        args: ['-e', 'setInterval(() => {}, 1000)'],
        cwd: process.cwd(),
        envs: process.env,
        timeoutMs: 200,
      }),
    ).rejects.toThrow(/timed out after 200ms/);
  });

  it.skipIf(process.platform !== 'darwin')(
    'stops the direct child on a hard deadline when ps cannot start',
    async () => {
      const cwd = mkdtempSync(join(tmpdir(), 'vp-denied-process-query-'));
      const pidFile = join(cwd, 'pid');
      const exec = vi.mocked(execFile).mockImplementationOnce(denyProcessUtility);
      try {
        await expect(
          runCommandSilently({
            command: process.execPath,
            args: [
              '-e',
              `require('node:fs').writeFileSync('pid', String(process.pid));
              setTimeout(() => {}, 3500);`,
            ],
            cwd,
            envs: process.env,
            timeoutMs: 1200,
          }),
        ).rejects.toThrow('Command timed out after 1200ms');
        expect(exec.mock.calls.map(([command]) => command)).toEqual(['ps']);
        const pid = Number(readFileSync(pidFile, 'utf8'));
        await expect.poll(() => fixtureChildExists(pid)).toBe(false);
      } finally {
        exec.mockReset();
        stopFixtureChild(pidFile);
        rmSync(cwd, { recursive: true, force: true });
      }
    },
  );

  it.skipIf(process.platform !== 'darwin')(
    'stops the idle command tree before returning when ps cannot start',
    async () => {
      const cwd = mkdtempSync(join(tmpdir(), 'vp-denied-process-tree-'));
      const pidFile = join(cwd, 'pid');
      const exec = vi.mocked(execFile).mockImplementationOnce(denyProcessUtility);
      const spawnSync = vi.spyOn(crossSpawn, 'sync').mockImplementationOnce(denyProcessUtility);
      const grandchild = `
        setTimeout(() => require('node:fs').writeFileSync('finished', ''), 1800);
      `;
      try {
        const result = await runCommandSilently({
          command: process.execPath,
          args: [
            '-e',
            `const child = require('node:child_process').spawn(
              process.execPath, ['-e', ${JSON.stringify(grandchild)}], { stdio: 'inherit' }
            );
            child.on('spawn', () => {
              require('node:fs').writeFileSync('pid', String(child.pid));
              console.log('resolving');
            });`,
          ],
          cwd,
          envs: process.env,
          idleTimeoutMs: 200,
          shouldWatchIdle: () => true,
          waitForPackageFetch: true,
        });
        expect(result).toMatchObject({
          exitCode: 124,
          idleTimedOut: true,
          stdout: Buffer.from('resolving\n'),
        });
        expect(exec.mock.calls.map(([command]) => command)).toContain('/usr/sbin/lsof');
        expect(spawnSync).toHaveBeenCalledWith(
          '/usr/sbin/lsof',
          expect.anything(),
          expect.anything(),
        );
        const pid = Number(readFileSync(pidFile, 'utf8'));
        await expect.poll(() => fixtureChildExists(pid)).toBe(false);
        expect(existsSync(join(cwd, 'finished'))).toBe(false);
      } finally {
        exec.mockReset();
        spawnSync.mockRestore();
        stopFixtureChild(pidFile);
        rmSync(cwd, { recursive: true, force: true });
      }
    },
  );

  it.skipIf(process.platform !== 'darwin')(
    'lets the command finish when process queries are unavailable',
    async () => {
      const exec = vi.mocked(execFile).mockImplementation(denyProcessUtility);
      try {
        const result = await runCommandSilently({
          command: process.execPath,
          args: ['-e', 'console.log("resolving"); setTimeout(() => console.log("done"), 600);'],
          cwd: process.cwd(),
          envs: process.env,
          idleTimeoutMs: 150,
          shouldWatchIdle: () => true,
          waitForPackageFetch: true,
        });
        expect(result.exitCode).toBe(0);
        expect(result.idleTimedOut).toBeUndefined();
        expect(result.stdout.toString()).toBe('resolving\ndone\n');
        expect(exec.mock.calls.map(([command]) => command)).toContain('/usr/sbin/lsof');
      } finally {
        exec.mockReset();
      }
    },
  );

  it.skipIf(process.platform === 'win32')(
    'enforces the deadline after a wrapper exits with inherited pipes',
    async () => {
      const cwd = mkdtempSync(join(tmpdir(), 'vp-deadline-command-'));
      const pidFile = join(cwd, 'pid');
      const start = performance.now();
      try {
        await expect(
          runCommandSilently({
            command: process.execPath,
            args: [
              '-e',
              `const child = require('node:child_process').spawn(
              process.execPath, ['-e', 'setTimeout(() => {}, 5000)'], { stdio: 'inherit' }
            );
            require('node:fs').writeFileSync('pid', String(child.pid));
            child.unref();`,
            ],
            cwd,
            envs: process.env,
            timeoutMs: 1200,
          }),
        ).rejects.toThrow(/timed out after 1200ms/);
        expect(existsSync(pidFile)).toBe(true);
        expect(performance.now() - start).toBeLessThan(3500);
      } finally {
        stopFixtureChild(pidFile);
        rmSync(cwd, { recursive: true, force: true });
      }
    },
  );

  it.skipIf(process.platform !== 'win32')(
    'enforces the deadline and stops a running pnpm.cmd wrapper and its child',
    async () => {
      const cwd = mkdtempSync(join(tmpdir(), 'vp-deadline-cmd-'));
      const pidFile = join(cwd, 'pid');
      const wrapper = join(cwd, 'pnpm.cmd');
      try {
        writeFileSync(wrapper, '@echo off\r\n"%VP_COMMAND_TEST_NODE%" "%~dp0hold.cjs"\r\n');
        writeFileSync(
          join(cwd, 'hold.cjs'),
          "require('node:fs').writeFileSync('pid', String(process.pid));\nsetInterval(() => {}, 1000);",
        );
        const start = performance.now();
        await expect(
          runCommandSilently({
            command: wrapper,
            args: ['--version'],
            cwd,
            envs: { ...process.env, VP_COMMAND_TEST_NODE: process.execPath },
            timeoutMs: 1500,
          }),
        ).rejects.toThrow(/timed out after 1500ms/);
        expect(performance.now() - start).toBeLessThan(4500);
        const pid = Number(readFileSync(pidFile, 'utf8'));
        await expect
          .poll(
            () => {
              try {
                process.kill(pid, 0);
                return false;
              } catch (error) {
                if ((error as NodeJS.ErrnoException).code === 'ESRCH') {
                  return true;
                }
                throw error;
              }
            },
            { timeout: 2500 },
          )
          .toBe(true);
      } finally {
        stopFixtureChild(pidFile);
        // The deadline rejects while taskkill and Windows handle cleanup finish.
        await rm(cwd, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 });
      }
    },
  );

  it('does not reject a fast child because a timeout is configured', async () => {
    const result = await runCommandSilently({
      command: process.execPath,
      args: ['-e', 'process.exit(3)'],
      cwd: process.cwd(),
      envs: process.env,
      timeoutMs: 30_000,
    });
    expect(result.exitCode).toBe(3);
  });

  it('waits for an install that keeps making progress past its idle deadline', async () => {
    const result = await runCommandSilently({
      command: process.execPath,
      args: [
        '-e',
        `
          let count = 0;
          const progress = setInterval(() => {
            const stream = count % 2 === 0 ? process.stdout : process.stderr;
            stream.write('step ' + count + '\\n');
            if (++count === 8) clearInterval(progress);
          }, 150);
        `,
      ],
      cwd: process.cwd(),
      envs: process.env,
      idleTimeoutMs: 1000,
    });
    expect(result.exitCode).toBe(0);
    expect(result.idleTimedOut).toBeUndefined();
    expect(result.stdout.toString()).toContain('step 6');
    expect(result.stderr.toString()).toContain('step 7');
  });

  it.each([false, true])('times watched stdout stages (file: %s)', async (stdoutToFile) => {
    const observedLines: string[] = [];
    const result = await runCommandSilently({
      command: process.execPath,
      args: [
        '-e',
        `
          setTimeout(() => {
            process.stdout.write('resolution_');
            setTimeout(() => {
              console.log('started');
              console.log('resolution_done');
              setTimeout(() => console.log('quiet script finished'), 1200);
            }, 20);
          }, 1200);
        `,
      ],
      cwd: process.cwd(),
      envs: process.env,
      idleTimeoutMs: 1000,
      stdoutToFile,
      shouldWatchIdle: (line) => {
        observedLines.push(line);
        return line === 'resolution_started';
      },
    });
    expect(result.exitCode).toBe(0);
    expect(result.idleTimedOut).toBeUndefined();
    expect(observedLines).toEqual([
      'resolution_started',
      'resolution_done',
      'quiet script finished',
    ]);
  });

  it.each([false, true])('allows scheduled retries (file: %s)', async (stdoutToFile) => {
    const result = await runCommandSilently({
      command: process.execPath,
      args: [
        '-e',
        `console.log('long wait');
          setTimeout(() => console.log('short wait'), 100);
          setTimeout(() => console.log('retried'), 700);
          setInterval(() => {}, 1000);`,
      ],
      cwd: process.cwd(),
      envs: process.env,
      idleTimeoutMs: 220,
      stdoutToFile,
      shouldWatchIdle: (line, deferIdleTimeout) => {
        if (line === 'long wait') {
          deferIdleTimeout(700);
        } else if (line === 'short wait') {
          deferIdleTimeout(150);
        }
        return true;
      },
    });
    expect(result.stdout.toString()).toContain('retried');
    expect(result).toMatchObject({ exitCode: 124, idleTimedOut: true });
  });

  it.skipIf(process.platform !== 'win32')(
    'expires an idle command with an empty TCP query on Windows',
    async () => {
      const result = await runCommandSilently({
        command: process.execPath,
        args: ['-e', 'console.log("resolving"); setTimeout(() => process.exit(7), 10_000);'],
        cwd: process.cwd(),
        envs: process.env,
        idleTimeoutMs: 500,
        shouldWatchIdle: () => true,
        waitForPackageFetch: true,
      });
      expect(result).toMatchObject({ exitCode: 124, idleTimedOut: true });
      expect(result.stdout.toString()).toBe('resolving\n');
    },
    15_000,
  );

  it('gives resolution time to resume after a quiet Git subprocess', async () => {
    const quietMs = process.platform === 'win32' ? 3500 : 1600;
    const result = await runCommandSilently({
      command: process.execPath,
      args: [
        '-e',
        `
          const git = require('node:child_process').spawn('git', ['hash-object', '--stdin']);
          git.on('spawn', () => {
            console.log('resolving');
            setTimeout(() => git.stdin.end('fixture'), ${quietMs});
          });
          git.on('close', (code) => setTimeout(() => console.log('git exited ' + code), 850));
          setInterval(() => {}, 1000);
        `,
      ],
      cwd: process.cwd(),
      envs: process.env,
      idleTimeoutMs: 1000,
      shouldWatchIdle: () => true,
      waitForPackageFetch: true,
    });
    expect(result.stdout.toString()).toContain('git exited 0');
    expect(result).toMatchObject({ exitCode: 124, idleTimedOut: true });
  }, 15_000);

  it('lets a quiet package transfer finish before stopping idle resolution', async () => {
    const transferMs = process.platform === 'win32' ? 3500 : 2200;
    const server = createServer((socket) => {
      const send = setInterval(() => socket.write('chunk'), 150);
      const finish = globalThis.setTimeout(() => {
        clearInterval(send);
        socket.end();
      }, transferMs);
      socket.once('close', () => {
        clearInterval(send);
        clearTimeout(finish);
      });
    });
    try {
      const listening = once(server, 'listening');
      server.listen(0, '127.0.0.1');
      await listening;
      const address = server.address();
      if (!address || typeof address === 'string') {
        throw new Error('Expected a TCP server address');
      }
      const result = await runCommandSilently({
        command: process.execPath,
        args: [
          '-e',
          `
            const socket = require('node:net').connect(${address.port}, '127.0.0.1');
            socket.on('connect', () => console.log('resolving'));
            socket.on('end', () => console.log('transfer complete'));
            socket.resume();
            setInterval(() => {}, 1000);
          `,
        ],
        cwd: process.cwd(),
        envs: process.env,
        idleTimeoutMs: 900,
        shouldWatchIdle: () => true,
        waitForPackageFetch: true,
      });
      expect(result.stdout.toString()).toContain('transfer complete');
      expect(result).toMatchObject({ exitCode: 124, idleTimedOut: true });
    } finally {
      server.close();
    }
  }, 18_000);

  it.each([false, true])('stops an idle tree (file: %s)', async (stdoutToFile) => {
    const cwd = mkdtempSync(join(tmpdir(), 'vp-install-command-'));
    const finished = join(cwd, 'finished');
    const spawnSync = vi.spyOn(crossSpawn, 'sync');
    const manager = `
      process.stderr.write('resolving packages\\n');
      setTimeout(() => {
        require('node:fs').writeFileSync('finished', '');
      }, 2200);
    `;
    try {
      const result = await runCommandSilently({
        command: process.execPath,
        args: [
          '-e',
          `console.log('install started');
          require('node:child_process').spawn(process.execPath, ['-e', ${JSON.stringify(manager)}], {
            stdio: 'inherit'
          });`,
        ],
        cwd,
        envs: process.env,
        idleTimeoutMs: 1000,
        stdoutToFile,
        shouldWatchIdle: (line) => line === 'install started',
      });
      expect(result.exitCode).toBe(124);
      expect(result.idleTimedOut).toBe(true);
      expect(result.stdout.toString()).toContain('install started');
      expect(result.stderr.toString()).toContain('resolving packages');
      await setTimeout(1500);
      expect(existsSync(finished)).toBe(false);
      if (process.platform === 'linux') {
        expect(spawnSync).not.toHaveBeenCalledWith('ps', expect.anything(), expect.anything());
      }
    } finally {
      spawnSync.mockRestore();
      rmSync(cwd, { recursive: true, force: true });
    }
  });

  it.skipIf(process.platform === 'win32').each([
    ['idle timeout', { idleTimeoutMs: 30_000 }],
    ['hard deadline', { timeoutMs: 30_000 }],
  ] as const)(
    'keeps captured commands with a %s in the caller process group',
    async (_name, timeout) => {
      const cwd = mkdtempSync(join(tmpdir(), 'vp-cancel-command-'));
      const ready = join(cwd, 'ready');
      const program = `
      const { writeFileSync } = require('node:fs');
      process.on('SIGINT', () => {
        writeFileSync('cleaned', '');
        process.exit(0);
      });
      writeFileSync('ready', String(process.pid));
      setTimeout(() => {}, 10000);
    `;
      const commandModule = new URL('../command.ts', import.meta.url).href;
      const runner = spawn(
        process.execPath,
        [
          '--input-type=module',
          '-e',
          `import { runCommandSilently } from ${JSON.stringify(commandModule)};
        await runCommandSilently({
          command: process.execPath,
          args: ['-e', ${JSON.stringify(program)}],
          cwd: process.cwd(),
          envs: process.env,
          ...${JSON.stringify(timeout)},
        });`,
        ],
        { cwd, stdio: 'ignore', detached: true },
      );
      const exited = once(runner, 'exit');
      const runnerPid = runner.pid;
      try {
        if (runnerPid === undefined) {
          throw new Error('The test process did not start');
        }
        await expect.poll(() => readFileSync(ready, 'utf8'), { timeout: 3000 }).toMatch(/^\d+$/);
        const childPid = readFileSync(ready, 'utf8');
        const childGroup = fixtureProcessGroup(childPid);
        process.kill(-runnerPid, 'SIGINT');
        await expect(exited).resolves.toEqual([null, 'SIGINT']);
        await expect.poll(() => existsSync(join(cwd, 'cleaned'))).toBe(true);
        expect(childGroup).toBe(runnerPid);
      } finally {
        if (runnerPid && runner.exitCode === null && runner.signalCode === null) {
          process.kill(-runnerPid, 'SIGTERM');
        }
        rmSync(cwd, { recursive: true, force: true });
      }
    },
  );

  it.skipIf(process.platform === 'win32')(
    'preserves the signal exit code for captured commands',
    async () => {
      const result = await runCommandSilently({
        command: process.execPath,
        args: ['-e', 'process.kill(process.pid, "SIGILL")'],
        cwd: process.cwd(),
        envs: process.env,
      });
      expect(result.exitCode).toBe(132);
    },
  );

  it.skipIf(process.platform === 'win32')(
    'preserves the signal exit code for inherited commands',
    async () => {
      const result = await runCommand({
        command: process.execPath,
        args: ['-e', 'process.kill(process.pid, "SIGILL")'],
        cwd: process.cwd(),
        envs: process.env,
      });
      expect(result.exitCode).toBe(132);
    },
  );
});
