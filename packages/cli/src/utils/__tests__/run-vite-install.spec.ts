import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { beforeEach, describe, expect, it, vi } from 'vitest';

import { PackageManager } from '../../types/index.ts';

vi.mock('../command.ts', () => ({
  runCommandSilently: vi.fn(),
}));

import { runCommandSilently } from '../command.ts';
import { runViteInstall } from '../prompts.ts';

const mockRun = vi.mocked(runCommandSilently);

function installResult(exitCode: number, stdout = '', stderr = '') {
  return { exitCode, stdout: Buffer.from(stdout), stderr: Buffer.from(stderr) };
}

describe('runViteInstall', () => {
  beforeEach(() => {
    mockRun.mockReset();
    delete process.env.VP_SKIP_INSTALL;
    delete process.env.VP_VERSION;
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
    expect(mockRun).toHaveBeenCalledWith(expect.objectContaining({ args: ['install'] }));
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
      expect.objectContaining({ args: ['install', '--ignore-scripts'] }),
    );
  });

  it('temporarily relaxes an npm age gate when the pinned npm lacks exclusions', async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'vite-plus-install-policy-'));
    try {
      fs.writeFileSync(path.join(dir, '.npmrc'), 'min-release-age=3\n');
      mockRun.mockResolvedValue(installResult(0));

      await runViteInstall(dir, false, undefined, {
        silent: true,
        packageManager: PackageManager.npm,
        packageManagerVersion: '11.12.1',
      });

      expect(mockRun).toHaveBeenCalledWith(
        expect.objectContaining({
          envs: expect.objectContaining({ NPM_CONFIG_MIN_RELEASE_AGE: '0' }),
        }),
      );
      expect(fs.readFileSync(path.join(dir, '.npmrc'), 'utf8')).not.toContain(
        'min-release-age-exclude',
      );
    } finally {
      fs.rmSync(dir, { recursive: true, force: true });
    }
  });
});
