import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it, vi } from 'vitest';

import type { FormatRunner } from '../format.ts';
import { canFormatWithOxfmt, collectChangedFormatPaths, formatMigratedProject } from '../format.ts';
import { createMigrationReport } from '../report.ts';

describe('formatMigratedProject', () => {
  it('formats the project root', async () => {
    const format = vi.fn<FormatRunner>().mockResolvedValue({
      durationMs: 1,
      exitCode: 0,
      status: 'formatted',
    });
    const report = createMigrationReport();
    const collectPaths = vi
      .fn<typeof collectChangedFormatPaths>()
      .mockResolvedValue(['package.json', 'vite.config.ts']);
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
      const format = vi.fn<FormatRunner>().mockResolvedValue({
        durationMs: 1,
        exitCode: 0,
        status: 'formatted',
      });
      const report = createMigrationReport();

      await expect(
        formatMigratedProject('/different/project', false, report, {
          format,
          collectPaths: vi
            .fn<typeof collectChangedFormatPaths>()
            .mockResolvedValue(['package.json']),
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

  it('splits a very large changed-file set across multiple format invocations', async () => {
    const format = vi
      .fn<FormatRunner>()
      .mockResolvedValue({ durationMs: 1, exitCode: 0, status: 'formatted' });
    const report = createMigrationReport();
    const paths = Array.from(
      { length: 5000 },
      (_, index) => `packages/app/src/very/deeply/nested/module-${index}.ts`,
    );
    const collectPaths = vi.fn<typeof collectChangedFormatPaths>().mockResolvedValue(paths);

    await expect(
      formatMigratedProject('/project', false, report, { format, collectPaths }),
    ).resolves.toBe(true);
    // A single spawn with all 5000 paths would risk E2BIG; it must be batched.
    expect(format.mock.calls.length).toBeGreaterThan(1);
    // Every path is still formatted exactly once across the batches.
    expect(format.mock.calls.flatMap((call) => call[2])).toEqual(paths);
    expect(report.warnings).toEqual([]);
  });

  it('skips formatting when migration changed no supported files', async () => {
    const format = vi.fn<FormatRunner>();
    const report = createMigrationReport();
    const collectPaths = vi.fn<typeof collectChangedFormatPaths>().mockResolvedValue([]);

    await expect(
      formatMigratedProject('/project', false, report, { format, collectPaths }),
    ).resolves.toBe(true);
    expect(format).not.toHaveBeenCalled();
    expect(report.warnings).toEqual([]);
  });

  it.each(['git', 'no-git'])('preserves standalone Renovate configs (%s)', async (mode) => {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-migrate-format-renovate-'));
    try {
      if (mode === 'git') {
        execFileSync('git', ['init'], { cwd: root, stdio: 'ignore' });
      }
      const source = "{extends:['config:recommended','group:vitePlus'],}";
      fs.mkdirSync(path.join(root, '.github'));
      fs.mkdirSync(path.join(root, 'nested'));
      const configs = ['renovate.json', '.github/renovate.json5'];
      for (const file of [...configs, 'nested/renovate.json5']) {
        fs.writeFileSync(path.join(root, file), source);
      }
      fs.writeFileSync(path.join(root, 'package.json'), '{"name":"demo"}');

      const oxfmt = fileURLToPath(
        new URL('./bin/oxfmt', import.meta.resolve('oxfmt/package.json')),
      );
      const formatted = await formatMigratedProject(root, false, createMigrationReport(), {
        format: async (cwd, _interactive, paths) => {
          execFileSync(process.execPath, [oxfmt, ...(paths ?? [])], { cwd, stdio: 'pipe' });
          return { status: 'formatted', exitCode: 0, durationMs: 0 };
        },
      });
      expect(formatted).toBe(true);
      for (const file of configs) {
        expect(fs.readFileSync(path.join(root, file), 'utf8')).toBe(source);
      }
      expect(fs.readFileSync(path.join(root, 'nested/renovate.json5'), 'utf8')).not.toBe(source);
      expect(fs.readFileSync(path.join(root, 'package.json'), 'utf8')).not.toBe('{"name":"demo"}');
    } finally {
      fs.rmSync(root, { recursive: true, force: true });
    }
  });

  it('reports a formatter nonzero exit without throwing', async () => {
    const format = vi.fn<FormatRunner>().mockResolvedValue({
      durationMs: 1,
      exitCode: 1,
      status: 'failed',
    });
    const report = createMigrationReport();
    // Non-git project: whole-project formatting.
    const collectPaths = vi.fn<typeof collectChangedFormatPaths>().mockResolvedValue(undefined);

    await expect(
      formatMigratedProject('/project', false, report, { format, collectPaths }),
    ).resolves.toBe(false);
    expect(report.warnings).toEqual([
      'Automatic formatting failed. Run `vp fmt` manually after migration.',
    ]);
  });

  it('reports a formatter exception without throwing', async () => {
    const format = vi.fn<FormatRunner>().mockRejectedValue(new Error('could not load config'));
    const report = createMigrationReport();
    const collectPaths = vi.fn<typeof collectChangedFormatPaths>().mockResolvedValue(undefined);

    await expect(
      formatMigratedProject('/project', false, report, { format, collectPaths }),
    ).resolves.toBe(false);
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
