import { describe, expect, it, vi } from 'vitest';

import { formatMigratedProject } from '../format.ts';
import { createMigrationReport } from '../report.ts';

describe('formatMigratedProject', () => {
  it('formats the project root', async () => {
    const format = vi.fn().mockResolvedValue({
      durationMs: 1,
      exitCode: 0,
      status: 'formatted',
    });
    const report = createMigrationReport();

    await expect(formatMigratedProject('/project', false, report, format)).resolves.toBe(true);
    expect(format).toHaveBeenCalledWith('/project', false, undefined, {
      silent: false,
      command: process.execPath,
      commandArgs: [...process.execArgv, process.argv[1]],
    });
    expect(report.warnings).toEqual([]);
  });

  it('reports a formatter nonzero exit without throwing', async () => {
    const format = vi.fn().mockResolvedValue({
      durationMs: 1,
      exitCode: 1,
      status: 'failed',
    });
    const report = createMigrationReport();

    await expect(formatMigratedProject('/project', false, report, format)).resolves.toBe(false);
    expect(report.warnings).toEqual([
      'Automatic formatting failed. Run `vp fmt` manually after migration.',
    ]);
  });

  it('reports a formatter exception without throwing', async () => {
    const format = vi.fn().mockRejectedValue(new Error('could not load config'));
    const report = createMigrationReport();

    await expect(formatMigratedProject('/project', false, report, format)).resolves.toBe(false);
    expect(report.warnings).toEqual([
      'Automatic formatting failed. Run `vp fmt` manually after migration.',
    ]);
  });
});
