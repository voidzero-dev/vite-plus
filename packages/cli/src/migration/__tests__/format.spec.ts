import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { describe, expect, it, vi } from 'vitest';

import { canFormatWithOxfmt, collectChangedFormatPaths, formatMigratedProject } from '../format.ts';
import { createMigrationReport } from '../report.ts';

describe('formatMigratedProject', () => {
  it('formats the project root', async () => {
    const format = vi.fn().mockResolvedValue({
      durationMs: 1,
      exitCode: 0,
      status: 'formatted',
    });
    const report = createMigrationReport();
    const collectPaths = vi.fn().mockResolvedValue(['package.json', 'vite.config.ts']);
    const excludedPaths = new Set(['notes.md']);

    await expect(
      formatMigratedProject('/project', false, report, {
        format,
        collectPaths,
        excludedPaths,
      }),
    ).resolves.toBe(true);
    expect(collectPaths).toHaveBeenCalledWith('/project', excludedPaths);
    expect(format).toHaveBeenCalledWith('/project', false, ['package.json', 'vite.config.ts'], {
      silent: false,
      command: process.execPath,
      commandArgs: [...process.execArgv, path.resolve(process.cwd(), process.argv[1])],
    });
    expect(report.warnings).toEqual([]);
  });

  it('resolves a relative CLI entry before formatting from the project root', async () => {
    const originalCliEntry = process.argv[1];
    process.argv[1] = './packages/cli/src/migration/bin.ts';
    try {
      const format = vi.fn().mockResolvedValue({
        durationMs: 1,
        exitCode: 0,
        status: 'formatted',
      });
      const report = createMigrationReport();

      await expect(
        formatMigratedProject('/different/project', false, report, {
          format,
          collectPaths: vi.fn().mockResolvedValue(['package.json']),
        }),
      ).resolves.toBe(true);
      expect(format).toHaveBeenCalledWith('/different/project', false, ['package.json'], {
        silent: false,
        command: process.execPath,
        commandArgs: [
          ...process.execArgv,
          path.resolve(process.cwd(), './packages/cli/src/migration/bin.ts'),
        ],
      });
    } finally {
      process.argv[1] = originalCliEntry;
    }
  });

  it('skips formatting when migration changed no supported files', async () => {
    const format = vi.fn();
    const report = createMigrationReport();
    const collectPaths = vi.fn().mockResolvedValue([]);

    await expect(
      formatMigratedProject('/project', false, report, { format, collectPaths }),
    ).resolves.toBe(true);
    expect(format).not.toHaveBeenCalled();
    expect(report.warnings).toEqual([]);
  });

  it('reports a formatter nonzero exit without throwing', async () => {
    const format = vi.fn().mockResolvedValue({
      durationMs: 1,
      exitCode: 1,
      status: 'failed',
    });
    const report = createMigrationReport();

    await expect(formatMigratedProject('/project', false, report, { format })).resolves.toBe(false);
    expect(report.warnings).toEqual([
      'Automatic formatting failed. Run `vp fmt` manually after migration.',
    ]);
  });

  it('reports a formatter exception without throwing', async () => {
    const format = vi.fn().mockRejectedValue(new Error('could not load config'));
    const report = createMigrationReport();

    await expect(formatMigratedProject('/project', false, report, { format })).resolves.toBe(false);
    expect(report.warnings).toEqual([
      'Automatic formatting failed. Run `vp fmt` manually after migration.',
    ]);
  });
});

describe('collectChangedFormatPaths', () => {
  it('collects existing changed Git paths without an extension allowlist', async () => {
    const projectRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-migrate-format-'));
    try {
      execFileSync('git', ['init'], { cwd: projectRoot, stdio: 'ignore' });
      fs.writeFileSync(path.join(projectRoot, 'package.json'), '{}\n');
      fs.writeFileSync(path.join(projectRoot, 'template.mdx'), '# untouched\n');
      fs.writeFileSync(path.join(projectRoot, 'bun.lock'), 'lockfileVersion = 1\n');
      execFileSync('git', ['add', '.'], { cwd: projectRoot });
      execFileSync(
        'git',
        [
          '-c',
          'user.name=Vite+ Test',
          '-c',
          'user.email=test@vite-plus.dev',
          'commit',
          '-m',
          'initial',
        ],
        { cwd: projectRoot, stdio: 'ignore' },
      );

      fs.writeFileSync(path.join(projectRoot, 'notes.md'), '# existing work\n');
      const preExistingPaths = await collectChangedFormatPaths(projectRoot);
      expect(preExistingPaths).toEqual(['notes.md']);

      fs.appendFileSync(path.join(projectRoot, 'package.json'), '\n');
      fs.writeFileSync(path.join(projectRoot, 'vite.config.ts'), 'export default {}\n');
      fs.writeFileSync(path.join(projectRoot, 'future.custom'), 'future format\n');

      await expect(
        collectChangedFormatPaths(projectRoot, new Set(preExistingPaths)),
      ).resolves.toEqual(['future.custom', 'package.json', 'vite.config.ts']);
    } finally {
      fs.rmSync(projectRoot, { recursive: true, force: true });
    }
  });

  it('falls back to full-project formatting outside Git', async () => {
    const projectRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-migrate-format-no-git-'));
    try {
      await expect(collectChangedFormatPaths(projectRoot)).resolves.toBeUndefined();
    } finally {
      fs.rmSync(projectRoot, { recursive: true, force: true });
    }
  });
});

describe('canFormatWithOxfmt', () => {
  it('formats projects that do not use Prettier', () => {
    expect(canFormatWithOxfmt(false, false)).toBe(true);
  });

  it('formats projects after Prettier was migrated', () => {
    expect(canFormatWithOxfmt(true, true)).toBe(true);
  });

  it('does not reformat projects that still use Prettier', () => {
    expect(canFormatWithOxfmt(true, false)).toBe(false);
  });
});
