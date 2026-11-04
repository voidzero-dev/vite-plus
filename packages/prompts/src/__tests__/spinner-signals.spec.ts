import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

import { build } from 'tsdown';
import { afterAll, beforeAll, describe, expect, it } from 'vite-plus/test';

// Windows does not deliver these signals to Node listeners like POSIX does.
describe.skipIf(process.platform === 'win32')('spinner process signals', () => {
  let directory: string;
  let spinnerUrl: string;

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
    spinnerUrl = pathToFileURL(join(directory, 'spinner.mjs')).href;
  });

  afterAll(() => {
    if (directory) {
      rmSync(directory, { recursive: true, force: true });
    }
  });

  describe.each([1, 2])('%i active spinners', (count) => {
    describe.each(['SIGINT', 'SIGTERM'] as const)('%s', (signal) => {
      it.each([
        { method: 'none', position: 0 },
        ...['on', 'once'].flatMap((method) =>
          Array.from({ length: count + 1 }, (_, position) => ({ method, position })),
        ),
      ])(
        'preserves shutdown with $method at position $position',
        async ({ method, position }) => {
          const child = spawn(
            process.execPath,
            [
              '--input-type=module',
              '--eval',
              `
                import assert from 'node:assert/strict';
                import { setTimeout as delay } from 'node:timers/promises';
                import { spinner } from ${JSON.stringify(spinnerUrl)};

                const spinners = [];
                const keepAlive = setInterval(() => {}, 1000);
                let calls = 0;
                const shutdown = async () => {
                  if (++calls > 1) process.exit(42);
                  await delay(30);
                  assert.ok(spinners.every(s => s.isCancelled));
                  process.stdout.write('cleanup complete\\n');
                  clearInterval(keepAlive);
                  process.disconnect();
                };
                for (let i = 0; i <= ${count}; i++) {
                  if (i === ${position} && ${JSON.stringify(method)} !== 'none') {
                    process[${JSON.stringify(method)}](${JSON.stringify(signal)}, shutdown);
                  }
                  if (i < ${count}) {
                    const s = spinner();
                    s.start('Working');
                    spinners.push(s);
                  }
                }
                process.send('ready');
              `,
            ],
            { stdio: ['ignore', 'pipe', 'pipe', 'ipc'], timeout: 5000, killSignal: 'SIGKILL' },
          );
          const closed = once(child, 'close');
          let stdout = '';
          let stderr = '';
          child.stdout!.on('data', (data) => {
            stdout += data;
          });
          child.stderr!.on('data', (data) => {
            stderr += data;
          });
          child.once('message', () => child.kill(signal));
          const [code, exitSignal] = await closed;
          expect(stderr).toBe('');
          if (method === 'none') {
            expect(exitSignal).toBe(signal);
            expect(code).toBeNull();
            expect(stdout).not.toContain('cleanup complete');
          } else {
            expect(exitSignal).toBeNull();
            expect(code).toBe(0);
            expect(stdout).toContain('cleanup complete');
          }
        },
        10000,
      );
    });
  });
});
