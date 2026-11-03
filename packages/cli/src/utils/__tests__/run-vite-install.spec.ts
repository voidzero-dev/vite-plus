import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { PackageManager } from '../../types/index.ts';

vi.mock(import('../command.ts'), async (importOriginal) => ({
  ...(await importOriginal()),
  runCommandSilently: vi.fn(),
}));

import { runCommandSilently } from '../command.ts';
import { runViteInstall } from '../prompts.ts';

const mockRun = vi.mocked(runCommandSilently);

function installResult(exitCode: number, stdout = '', stderr = '') {
  return { exitCode, stdout: Buffer.from(stdout), stderr: Buffer.from(stderr) };
}

describe('runViteInstall with detectIgnoredBuilds', () => {
  beforeEach(() => {
    mockRun.mockReset();
    vi.stubEnv('VP_SKIP_INSTALL', undefined);
    vi.stubEnv('VP_CLI_BIN', undefined);
  });

  afterEach(() => vi.unstubAllEnvs());

  it('preserves an explicit VP_CLI_BIN override and its arguments', async () => {
    vi.stubEnv('VP_CLI_BIN', '/custom/vp');
    mockRun.mockResolvedValue(installResult(0));
    await runViteInstall('/project', false, ['--offline'], { silent: true });
    expect(mockRun).toHaveBeenCalledWith(
      expect.objectContaining({ command: '/custom/vp', args: ['install', '--offline'] }),
    );
  });

  it('treats pnpm >= 11 ERR_PNPM_IGNORED_BUILDS exit-1 as a completed install', async () => {
    mockRun.mockResolvedValue(
      installResult(
        1,
        '[ERR_PNPM_IGNORED_BUILDS] Ignored build scripts: better-sqlite3@11.0.0, esbuild@0.25.0',
      ),
    );
    const summary = await runViteInstall('/project', false, undefined, {
      silent: true,
      packageManager: PackageManager.pnpm,
      packageManagerVersion: '11.6.0',
      detectIgnoredBuilds: true,
    });
    expect(summary.status).toBe('installed');
    expect(summary.pendingBuilds).toEqual(['better-sqlite3', 'esbuild']);
    // Detection mode must NOT add --ignore-scripts (that would hide the builds
    // from `approve-builds` afterwards).
    expect(mockRun).toHaveBeenCalledWith(
      expect.objectContaining({ command: process.execPath, args: [expect.any(String), 'install'] }),
    );
  });

  it('still reports a genuine non-zero exit (no ignored-builds error) as failed', async () => {
    mockRun.mockResolvedValue(installResult(1, 'ERR_PNPM_FETCH_404  GET https://...'));
    const summary = await runViteInstall('/project', false, undefined, {
      silent: true,
      packageManager: PackageManager.pnpm,
      packageManagerVersion: '11.6.0',
      detectIgnoredBuilds: true,
    });
    expect(summary.status).toBe('failed');
  });

  it('parses pendingBuilds from a clean pnpm 10 warning (exit 0)', async () => {
    mockRun.mockResolvedValue(installResult(0, 'Ignored build scripts: esbuild.\nDone in 171ms'));
    const summary = await runViteInstall('/project', false, undefined, {
      silent: true,
      packageManager: PackageManager.pnpm,
      packageManagerVersion: '10.16.1',
      detectIgnoredBuilds: true,
    });
    expect(summary.status).toBe('installed');
    expect(summary.pendingBuilds).toEqual(['esbuild']);
  });

  it('keeps the --ignore-scripts workaround (and no pendingBuilds) when detection is off', async () => {
    mockRun.mockResolvedValue(installResult(0, ''));
    const summary = await runViteInstall('/project', false, undefined, {
      silent: true,
      packageManager: PackageManager.pnpm,
      packageManagerVersion: '11.6.0',
    });
    expect(summary.status).toBe('installed');
    expect(summary.pendingBuilds).toBeUndefined();
    expect(mockRun).toHaveBeenCalledWith(
      expect.objectContaining({ args: [expect.any(String), 'install', '--ignore-scripts'] }),
    );
  });
});
