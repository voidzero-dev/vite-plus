import { execFileSync, spawn } from 'node:child_process';
import { once } from 'node:events';
import { existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { setTimeout } from 'node:timers/promises';

import { describe, expect, it } from 'vitest';

import { runCommand, runCommandSilently } from '../command.ts';

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

  it('only times the portion between complete stdout lines that start and stop the timer', async () => {
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

  it('stops an idle install tree and keeps partial output available for diagnosis', async () => {
    const cwd = mkdtempSync(join(tmpdir(), 'vp-install-command-'));
    const finished = join(cwd, 'finished');
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
        shouldWatchIdle: (line) => line === 'install started',
      });
      expect(result.exitCode).toBe(124);
      expect(result.idleTimedOut).toBe(true);
      expect(result.stdout.toString()).toContain('install started');
      expect(result.stderr.toString()).toContain('resolving packages');
      await setTimeout(1500);
      expect(existsSync(finished)).toBe(false);
    } finally {
      rmSync(cwd, { recursive: true, force: true });
    }
  });

  it.skipIf(process.platform === 'win32')(
    'lets Ctrl+C reach guarded children in the caller process group',
    async () => {
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
          idleTimeoutMs: 30_000,
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
        const groups = execFileSync('ps', ['-A', '-o', 'pid=', '-o', 'pgid='], {
          encoding: 'utf8',
        });
        const entry = groups.split('\n').find((line) => line.trim().split(/\s+/)[0] === childPid);
        process.kill(-runnerPid, 'SIGINT');
        await expect(exited).resolves.toEqual([null, 'SIGINT']);
        await expect.poll(() => existsSync(join(cwd, 'cleaned'))).toBe(true);
        expect(Number(entry?.trim().split(/\s+/)[1])).toBe(runnerPid);
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
