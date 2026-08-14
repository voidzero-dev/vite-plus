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

  it(
    'times out even when a grandchild inherits the stdio pipes',
    { timeout: 10_000 },
    async () => {
      // Arbitrary project code run by a config worker can spawn its own
      // children. This child spawns a grandchild that inherits the piped
      // stdio, then wedges like a blocking plugin factory. Without a tree
      // kill the SIGKILL reaches only the direct child, the grandchild
      // keeps the stdout pipe open, `close` never fires, and the promise
      // never settles. The grandchild self-terminates after 15s so a
      // failing run leaves nothing behind.
      await expect(
        runCommandSilently({
          command: process.execPath,
          args: [
            '-e',
            `const { spawn } = require('node:child_process');
             spawn(process.execPath, ['-e', 'setTimeout(() => {}, 15_000)'], { stdio: 'inherit' });
             setInterval(() => {}, 1000);`,
          ],
          cwd: process.cwd(),
          envs: process.env,
          timeoutMs: 500,
        }),
      ).rejects.toThrow(/timed out after 500ms/);
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
