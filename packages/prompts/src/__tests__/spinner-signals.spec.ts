import { type ChildProcess, spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

import { build } from 'tsdown';
import { afterAll, beforeAll, describe, expect, it } from 'vite-plus/test';

/**
 * Whether `pid` is still executing. An orphan that has ended stays in the
 * process table as a zombie where nothing reaps it, as in some containers.
 */
function isRunning(pid: number) {
  try {
    process.kill(pid, 0);
  } catch {
    return false;
  }
  try {
    const stat = readFileSync(`/proc/${pid}/stat`, 'utf8');
    // The state follows the command name, which can contain spaces and parentheses.
    return stat[stat.lastIndexOf(')') + 2] !== 'Z';
  } catch {
    // No procfs, as on macOS, where launchd reaps orphans.
    return true;
  }
}

// Windows has no signals or process groups; output.spec.ts covers its exit code.
describe.skipIf(process.platform === 'win32')('spinner interrupts', () => {
  let directory: string;
  let prelude: string;

  beforeAll(async () => {
    directory = mkdtempSync(join(tmpdir(), 'vp-spinner-signals-'));
    // Build current source rather than relying on potentially stale package output.
    await build({
      config: false,
      entry: [fileURLToPath(new URL('../spinner.ts', import.meta.url))],
      outDir: directory,
      outExtensions: () => ({ js: '.mjs' }),
      deps: { alwaysBundle: [/.*/], onlyBundle: false },
      dts: false,
      logLevel: 'silent',
    });
    // Synchronous writes, so output is not lost when a signal ends the process.
    prelude = `
      import { writeSync } from 'node:fs';
      import { Writable } from 'node:stream';
      import { spinner } from ${JSON.stringify(pathToFileURL(join(directory, 'spinner.mjs')).href)};
      const output = (isTTY) => Object.assign(
        new Writable({ write(chunk, _, done) { writeSync(1, chunk); done(); } }),
        { isTTY, columns: 80 },
      );
      const keepAlive = setInterval(() => {}, 1000);
    `;
  });

  afterAll(() => {
    if (directory) {
      rmSync(directory, { recursive: true, force: true });
    }
  });

  /**
   * Runs `script` in its own process group, because Ctrl+C signals the whole
   * group. `act` runs once the script sends its first message.
   */
  async function run(script: string, act: (child: ChildProcess, message: unknown) => void) {
    const child = spawn(process.execPath, ['--input-type=module', '--eval', prelude + script], {
      detached: true,
      env: { ...process.env, CI: '' },
      stdio: ['pipe', 'pipe', 'pipe', 'ipc'],
      timeout: 5000,
      killSignal: 'SIGKILL',
    });
    let stdout = '';
    let stderr = '';
    child.stdout!.on('data', (data) => {
      stdout += data;
    });
    child.stderr!.on('data', (data) => {
      stderr += data;
    });
    child.once('message', (message) => act(child, message));
    const [code, signal] = await once(child, 'close');
    return { code, signal, stdout, stderr };
  }

  it('ends the process group by SIGINT on Ctrl+C', async () => {
    let worker = 0;
    try {
      const result = await run(
        `
          import { spawn } from 'node:child_process';
          const worker = spawn(process.execPath, ['--eval', 'setInterval(() => {}, 1000)'], {
            stdio: 'ignore',
          });
          spinner({ output: output(true) }).start('Working');
          process.send(worker.pid);
        `,
        (child, pid) => {
          worker = pid as number;
          child.stdin!.write('\x03');
        },
      );
      expect(result).toMatchObject({ code: null, signal: 'SIGINT', stderr: '' });
      expect(result.stdout).toContain('Canceled');
      // A running package manager must not outlive the interrupted command.
      await expect.poll(() => isRunning(worker), { timeout: 2000 }).toBe(false);
    } finally {
      if (worker && isRunning(worker)) {
        process.kill(worker, 'SIGKILL');
      }
    }
  });

  it('forwards one Ctrl+C once while two spinners are active', async () => {
    const result = await run(
      `
        import { setTimeout as delay } from 'node:timers/promises';
        let calls = 0;
        process.on('SIGINT', async () => {
          if (++calls > 1) process.exit(42);
          await delay(50);
          writeSync(1, 'cleanup complete\\n');
          process.exit(0);
        });
        spinner({ output: output(true) }).start('First');
        spinner({ output: output(true) }).start('Second');
        process.send('ready');
      `,
      (child) => child.stdin!.write('\x03'),
    );
    expect(result).toMatchObject({ code: 0, signal: null, stderr: '' });
    expect(result.stdout).toContain('cleanup complete');
  });

  describe.each(['SIGINT', 'SIGTERM'] as const)('%s', (signal) => {
    it.each([1, 2])('ends the process by the signal with %i active spinners', async (count) => {
      const result = await run(
        `
          for (let i = 0; i < ${count}; i++) {
            spinner({ output: output(false) }).start('Working');
          }
          process.send('ready');
        `,
        (child) => child.kill(signal),
      );
      expect(result).toMatchObject({ code: null, signal, stderr: '' });
      expect(result.stdout).toContain('Canceled');
    });

    it('ends the process by the signal after another listener came and went', async () => {
      const result = await run(
        `
          const signal = ${JSON.stringify(signal)};
          const other = () => {};
          spinner({ output: output(false) }).start('Working');
          process.on(signal, other);
          process.off(signal, other);
          process.send('ready');
        `,
        (child) => child.kill(signal),
      );
      expect(result).toMatchObject({ code: null, signal, stderr: '' });
    });

    // The shutdown exits with 42 when the signal reaches it twice, and a signal
    // raised again during its cleanup ends the process before it reports.
    it.each([
      ['a listener registered before the spinner', 'process.on(signal, shutdown); start();'],
      ['a one-shot listener registered before it', 'process.once(signal, shutdown); start();'],
      [
        'a one-shot listener prepended while spinning',
        'start(); process.prependOnceListener(signal, shutdown);',
      ],
    ])('leaves the shutdown to %s', async (_, setup) => {
      const result = await run(
        `
          import { setTimeout as delay } from 'node:timers/promises';
          const signal = ${JSON.stringify(signal)};
          let active;
          let calls = 0;
          const start = () => {
            active = spinner({ output: output(false) });
            active.start('Working');
          };
          const shutdown = async () => {
            if (++calls > 1) process.exit(42);
            await delay(50);
            writeSync(1, 'cleanup complete, cancelled=' + active.isCancelled + '\\n');
            process.exit(0);
          };
          ${setup}
          process.send('ready');
        `,
        (child) => child.kill(signal),
      );
      expect(result).toMatchObject({ code: 0, signal: null, stderr: '' });
      expect(result.stdout).toContain('cleanup complete, cancelled=true');
    });
  });

  it('only stops the spinner on an emit without the signal name', async () => {
    const result = await run(
      `
        const active = spinner({ output: output(false) });
        active.start('Working');
        process.emit('SIGINT');
        setTimeout(() => {
          writeSync(1, 'still alive, cancelled=' + active.isCancelled + '\\n');
          process.exit(0);
        }, 50);
        process.send('ready');
      `,
      () => {},
    );
    expect(result).toMatchObject({ code: 0, signal: null, stderr: '' });
    expect(result.stdout).toContain('still alive, cancelled=true');
  });
});
