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
import { confirmTsupMigration, detectTsupProject, migrateTsupToTsdown } from '../migrator/tsup.ts';

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
          '--no-install',
        ],
      }),
    );
  });

  it('detects a workspace-only tsup config', () => {
    fs.writeFileSync(
      path.join(projectPath, 'package.json'),
      '{"name":"workspace","private":true}\n',
    );
    fs.unlinkSync(path.join(projectPath, 'tsup.config.ts'));
    const packagePath = path.join(projectPath, 'packages/a');
    fs.mkdirSync(packagePath, { recursive: true });
    fs.writeFileSync(
      path.join(packagePath, 'package.json'),
      '{"name":"a","devDependencies":{"tsup":"^8.5.0"}}\n',
    );
    fs.writeFileSync(path.join(packagePath, 'tsup.config.ts'), 'export default {};\n');

    expect(detectTsupProject(projectPath, [{ name: 'a', path: 'packages/a' }])).toEqual({
      hasDependency: true,
      hasConfig: true,
      configFile: undefined,
    });
  });

  it('restores all workspace targets when a later migration fails', async () => {
    fs.writeFileSync(
      path.join(projectPath, 'package.json'),
      '{"name":"workspace","private":true}\n',
    );
    fs.unlinkSync(path.join(projectPath, 'tsup.config.ts'));
    const packages = [
      { name: 'a', path: 'packages/a' },
      { name: 'b', path: 'packages/b' },
    ];
    const originalFiles = new Map<string, string>();

    for (const workspacePackage of packages) {
      const packagePath = path.join(projectPath, workspacePackage.path);
      const packageJson = `${JSON.stringify(
        {
          name: workspacePackage.name,
          scripts: { build: 'tsup' },
          devDependencies: { tsup: '^8.5.0' },
        },
        null,
        2,
      )}\n`;
      const tsupConfig = `export default { name: '${workspacePackage.name}' };\n`;
      fs.mkdirSync(packagePath, { recursive: true });
      fs.writeFileSync(path.join(packagePath, 'package.json'), packageJson);
      fs.writeFileSync(path.join(packagePath, 'tsup.config.ts'), tsupConfig);
      originalFiles.set(path.join(packagePath, 'package.json'), packageJson);
      originalFiles.set(path.join(packagePath, 'tsup.config.ts'), tsupConfig);
    }

    mockRunCommandSilently.mockImplementation(async ({ cwd }) => {
      const packageJsonPath = path.join(cwd, 'package.json');
      fs.writeFileSync(packageJsonPath, '{"name":"partially-migrated"}\n');
      fs.writeFileSync(path.join(cwd, 'tsdown.config.ts'), 'export default {};\n');
      fs.unlinkSync(path.join(cwd, 'tsup.config.ts'));
      return {
        exitCode: path.basename(cwd) === 'b' ? 1 : 0,
        stdout: Buffer.alloc(0),
        stderr: Buffer.alloc(0),
      };
    });

    await expect(
      migrateTsupToTsdown(projectPath, false, PackageManager.pnpm, undefined, packages, {
        silent: true,
      }),
    ).resolves.toBe(false);

    expect(mockRunCommandSilently).toHaveBeenCalledTimes(2);
    for (const [filePath, contents] of originalFiles) {
      expect(fs.readFileSync(filePath, 'utf8')).toBe(contents);
      expect(fs.existsSync(path.join(path.dirname(filePath), 'tsdown.config.ts'))).toBe(false);
    }
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
