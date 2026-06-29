import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../utils/command.ts', () => ({
  runCommandSilently: vi.fn(),
}));

import { runCommandSilently } from '../../utils/command.ts';
import { checkRolldownCompatibility, ROLLDOWN_COMPAT_RESULT_PREFIX } from '../compat/runner.ts';
import { createMigrationReport } from '../report.ts';

const mockRunCommandSilently = vi.mocked(runCommandSilently);

describe('checkRolldownCompatibility', () => {
  beforeEach(() => {
    mockRunCommandSilently.mockReset();
  });

  it('merges warnings returned by the isolated config worker', async () => {
    mockRunCommandSilently.mockResolvedValue({
      exitCode: 0,
      stdout: Buffer.from(
        `project config output\n${ROLLDOWN_COMPAT_RESULT_PREFIX}${JSON.stringify({ warnings: ['manualChunks warning'] })}\n`,
      ),
      stderr: Buffer.alloc(0),
    });
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
    });

    const [{ args }] = mockRunCommandSilently.mock.calls[0];
    expect(args[0]).not.toMatch(/compat[/\\]compat/);
  });

  it('skips compatibility checking when project config crashes the worker', async () => {
    mockRunCommandSilently.mockResolvedValue({
      exitCode: 7,
      stdout: Buffer.from(
        `${ROLLDOWN_COMPAT_RESULT_PREFIX}${JSON.stringify({ warnings: ['incomplete result'] })}\n`,
      ),
      stderr: Buffer.from('project config crashed'),
    });
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
