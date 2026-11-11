import { type ChildProcess, spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

import { build } from 'tsdown';
import { afterAll, beforeAll, describe, expect, it } from 'vite-plus/test';

function isAlive(pid: number) {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
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
      await expect.poll(() => isAlive(worker), { timeout: 2000 }).toBe(false);
    } finally {
      if (worker && isAlive(worker)) {
        process.kill(worker, 'SIGKILL');
      }
    }
  });
});
