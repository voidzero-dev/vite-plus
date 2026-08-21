import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const { mockConfirm, mockInfo } = vi.hoisted(() => ({
  mockConfirm: vi.fn(),
  mockInfo: vi.fn(),
}));

vi.mock('@voidzero-dev/vite-plus-prompts', () => ({
  confirm: mockConfirm,
  isCancel: () => false,
  log: {
    info: mockInfo,
    success: vi.fn(),
    warn: vi.fn(),
  },
}));

vi.mock('../../utils/command.ts', () => ({
  runCommandSilently: vi.fn(),
}));
vi.mock('../../utils/prompts.ts', () => ({
  cancelAndExit: vi.fn(),
}));

import { PackageManager } from '../../types/index.ts';
import { runCommandSilently } from '../../utils/command.ts';
import { TSDOWN_MIGRATE_VERSION, TSDOWN_MIGRATION_SKILL_URL } from '../../utils/constants.ts';
import { displayRelative } from '../../utils/path.ts';
import { confirmTsupMigration, migrateTsupToTsdown } from '../migrator/tsup.ts';

const mockRunCommandSilently = vi.mocked(runCommandSilently);

function manualMigrationOptions(targetLabel = 'the project root'): string {
  return [
    'Choose one of these manual migration methods:',
    `  1. Run \`vp dlx tsdown-migrate\` in ${targetLabel}.`,
    '  2. Use the tsdown migration skill:',
    `     ${TSDOWN_MIGRATION_SKILL_URL}`,
  ].join('\n');
}

describe('tsup migration', () => {
  let projectPath: string;

  beforeEach(() => {
    projectPath = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-test-tsup-'));
    fs.writeFileSync(
      path.join(projectPath, 'package.json'),
      `${JSON.stringify({ name: 'fixture', devDependencies: { tsup: '^8.5.0' } }, null, 2)}\n`,
    );
    fs.writeFileSync(
      path.join(projectPath, 'tsup.config.ts'),
      "import { defineConfig } from 'tsup';\nexport default defineConfig({ dts: true });\n",
    );
    mockRunCommandSilently.mockResolvedValue({
      exitCode: 0,
      stdout: Buffer.alloc(0),
      stderr: Buffer.alloc(0),
    });
    mockConfirm.mockResolvedValue(true);
  });

  afterEach(() => {
    fs.rmSync(projectPath, { recursive: true, force: true });
    mockRunCommandSilently.mockReset();
    mockConfirm.mockReset();
    mockInfo.mockReset();
  });

  it('passes the package manager as a separate CLI argument', async () => {
    await expect(
      migrateTsupToTsdown(projectPath, false, PackageManager.npm, 'tsup.config.ts', undefined, {
        silent: true,
      }),
    ).resolves.toBe(true);

    expect(mockRunCommandSilently).toHaveBeenCalledWith(
      expect.objectContaining({
        args: [
          'dlx',
          `tsdown-migrate@${TSDOWN_MIGRATE_VERSION}`,
          '--yes',
          '--package-manager',
          'npm',
        ],
      }),
    );
  });

  it('shows the migration skill when automatic migration is declined', async () => {
    mockConfirm.mockResolvedValue(false);

    await expect(confirmTsupMigration(true)).resolves.toBe(false);

    expect(mockInfo).toHaveBeenCalledWith(manualMigrationOptions());
  });

  it('shows the migration skill when automatic migration fails', async () => {
    mockRunCommandSilently.mockResolvedValue({
      exitCode: 1,
      stdout: Buffer.alloc(0),
      stderr: Buffer.alloc(0),
    });

    await expect(
      migrateTsupToTsdown(projectPath, false, PackageManager.npm, 'tsup.config.ts', undefined, {
        silent: true,
      }),
    ).resolves.toBe(false);

    expect(mockInfo).toHaveBeenCalledWith(
      `Automatic tsup migration failed.\n\n${manualMigrationOptions(
        displayRelative(projectPath) || 'the project root',
      )}\n`,
    );
  });
});
