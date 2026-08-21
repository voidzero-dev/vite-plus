import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../utils/command.ts', () => ({
  runCommandSilently: vi.fn(),
}));
vi.mock('../../utils/prompts.ts', () => ({
  cancelAndExit: vi.fn(),
}));

import { PackageManager } from '../../types/index.ts';
import { runCommandSilently } from '../../utils/command.ts';
import { migrateTsupToTsdown } from '../migrator/tsup.ts';

const mockRunCommandSilently = vi.mocked(runCommandSilently);

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
  });

  afterEach(() => {
    fs.rmSync(projectPath, { recursive: true, force: true });
    mockRunCommandSilently.mockReset();
  });

  it('passes the package manager as a separate CLI argument', async () => {
    await expect(
      migrateTsupToTsdown(projectPath, false, PackageManager.npm, 'tsup.config.ts', undefined, {
        silent: true,
      }),
    ).resolves.toBe(true);

    expect(mockRunCommandSilently).toHaveBeenCalledWith({
      command: 'vp',
      args: ['dlx', 'tsdown-migrate@0.23.0-rc.0', '--yes', '--package-manager', 'npm'],
      cwd: projectPath,
      envs: process.env,
    });
  });
});
