import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../utils/command.ts', () => ({
  runCommandSilently: vi.fn<typeof runCommandSilently>(),
}));

import { runCommandSilently } from '../../utils/command.ts';
import { LINT_CONFIG_RESULT_PREFIX } from '../compat/protocol.ts';
import {
  checkRolldownCompatibility,
  readLintConfigInWorker,
  ROLLDOWN_COMPAT_RESULT_PREFIX,
} from '../compat/runner.ts';
import { createMigrationReport } from '../report.ts';

const mockRunCommandSilently = vi.mocked(runCommandSilently);

function mockWorkerOutput(stdout: string, exitCode = 0): void {
  mockRunCommandSilently.mockResolvedValue({
    exitCode,
    stdout: Buffer.from(stdout),
    stderr: Buffer.alloc(0),
  });
}

beforeEach(() => {
  mockRunCommandSilently.mockReset();
});

describe('checkRolldownCompatibility', () => {
  it('merges warnings returned by the isolated config worker', async () => {
    mockWorkerOutput(
      `project config output\n${ROLLDOWN_COMPAT_RESULT_PREFIX}{"warnings":["manualChunks warning"]}\n`,
    );
    const report = createMigrationReport();

    await checkRolldownCompatibility('/project', report);

    expect(report.warnings).toEqual(['manualChunks warning']);
    expect(mockRunCommandSilently).toHaveBeenCalledWith({
      command: process.execPath,
      // fileURLToPath yields OS-native separators: '/' on POSIX, '\' on Windows.
      // The worker must resolve as the emitted sibling `migration/compat/worker.js`,
      // never the doubled `migration/compat/compat/worker.js` that `./compat/worker.js`
      // produces relative to this module's own directory.
      args: [expect.stringMatching(/migration[/\\]compat[/\\]worker\.js$/), '/project'],
      cwd: '/project',
      envs: process.env,
      // A wedged config worker (e.g. a blocking plugin factory in a project
      // whose local vite-plus predates the skip handshake) must not hang the
      // migration; the runner opts into the kill-after-timeout path.
      timeoutMs: expect.any(Number),
    });

    const [{ args }] = mockRunCommandSilently.mock.calls[0];
    expect(args[0]).not.toMatch(/compat[/\\]compat/);
  });

  it('skips compatibility checking when project config crashes the worker', async () => {
    mockWorkerOutput(`${ROLLDOWN_COMPAT_RESULT_PREFIX}{"warnings":["incomplete result"]}\n`, 7);
    const report = createMigrationReport();

    await expect(checkRolldownCompatibility('/project', report)).resolves.toBeUndefined();
    expect(report.warnings).toEqual([]);
  });

  it('skips compatibility checking when the worker cannot start', async () => {
    mockRunCommandSilently.mockRejectedValue(new Error('spawn failed'));
    const report = createMigrationReport();

    await expect(checkRolldownCompatibility('/project', report)).resolves.toBeUndefined();
    expect(report.warnings).toEqual([]);
  });
});

describe('readLintConfigInWorker', () => {
  it.each([null, { rules: { 'vite-plus/prefer-vite-plus-imports': 'off' } }])(
    'reads lint metadata %j after project output',
    async (lint) => {
      mockWorkerOutput(`project output\n${LINT_CONFIG_RESULT_PREFIX}${JSON.stringify({ lint })}\n`);
      expect(await readLintConfigInWorker('/project')).toEqual(lint ?? undefined);
      expect(mockRunCommandSilently).toHaveBeenCalledWith({
        command: process.execPath,
        args: [expect.stringMatching(/migration[/\\]compat[/\\]worker\.js$/), '/project', 'lint'],
        cwd: '/project',
        envs: process.env,
        timeoutMs: 30_000,
      });
    },
  );

  it('rejects a crashed worker even if it emitted metadata', async () => {
    mockWorkerOutput(`${LINT_CONFIG_RESULT_PREFIX}{"lint":null}\n`, 7);
    await expect(readLintConfigInWorker('/project')).rejects.toThrow('exited with code 7');
  });

  it.each(['', '{', '{}', '{"lint":[]}'])(
    'rejects missing or invalid metadata %j',
    async (json) => {
      mockWorkerOutput(json ? `${LINT_CONFIG_RESULT_PREFIX}${json}\n` : '');
      await expect(readLintConfigInWorker('/project')).rejects.toThrow(
        'did not return lint metadata',
      );
    },
  );

  it.each(['spawn failed', 'Command timed out'])('reports worker failure: %s', async (message) => {
    mockRunCommandSilently.mockRejectedValue(new Error(message));
    await expect(readLintConfigInWorker('/project')).rejects.toThrow(message);
  });
});
