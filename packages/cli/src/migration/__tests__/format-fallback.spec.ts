import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../utils/command.ts', () => ({
  runCommandSilently: vi.fn(),
}));

import { runCommandSilently } from '../../utils/command.ts';
import { collectChangedFormatPaths } from '../format.ts';

const mockRunCommandSilently = vi.mocked(runCommandSilently);

function result(exitCode: number, stdout = '') {
  return { exitCode, stdout: Buffer.from(stdout), stderr: Buffer.alloc(0) };
}

describe('collectChangedFormatPaths git fallback', () => {
  beforeEach(() => {
    mockRunCommandSilently.mockReset();
  });

  it('falls back to full-project formatting when not inside a Git worktree', async () => {
    mockRunCommandSilently.mockImplementation(({ args }) => {
      if (args[0] === 'rev-parse') {
        return Promise.resolve(result(128, ''));
      }
      throw new Error(`unexpected git ${args.join(' ')}`);
    });

    await expect(collectChangedFormatPaths('/project')).resolves.toBeUndefined();
    expect(mockRunCommandSilently).toHaveBeenCalledTimes(1);
  });

  it('skips formatting instead of reformatting the whole tree when Git cannot list changes', async () => {
    mockRunCommandSilently.mockImplementation(({ args }) => {
      if (args[0] === 'rev-parse') {
        return Promise.resolve(result(0, 'true\n'));
      }
      if (args[0] === 'diff' && !args.includes('--cached')) {
        // e.g. a locked repo or mid-rebase: the change enumeration errors.
        return Promise.resolve(result(128, ''));
      }
      return Promise.resolve(result(0, ''));
    });

    // Must be [] (skip), not undefined (which would reformat every file).
    await expect(collectChangedFormatPaths('/project')).resolves.toEqual([]);
  });
});
