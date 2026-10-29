import fs from 'node:fs';
import path from 'node:path';

import { afterEach, describe, expect, it, vi } from 'vitest';

import { PackageManager } from '../../types/index.ts';
import { resolveCreatePackageManager } from '../package-manager.ts';

const { run, download } = vi.hoisted(() => ({ run: vi.fn(), download: vi.fn() }));

vi.mock('../../../binding/index.js', () => ({ getVpDirs: () => ({ bin: '/vp/bin' }) }));
vi.mock('../../utils/command.ts', () => ({ runCommandSilently: run }));
vi.mock('../../utils/prompts.ts', () => ({ downloadPackageManager: download }));

afterEach(() => {
  vi.restoreAllMocks();
  vi.resetAllMocks();
  vi.unstubAllEnvs();
});

describe('resolveCreatePackageManager', () => {
  it('preserves local-only creation when no global CLI is installed', async () => {
    vi.stubEnv('VP_CLI_BIN', undefined);
    vi.spyOn(fs, 'existsSync').mockReturnValue(false);
    const manager = { name: 'npm', version: '11.6.0', binPrefix: '/npm/bin' };
    download.mockResolvedValue(manager);

    expect(await resolveCreatePackageManager(PackageManager.npm, '11.6.0')).toBe(manager);
    expect(run).not.toHaveBeenCalled();
    expect(download).toHaveBeenCalledWith(PackageManager.npm, '11.6.0', undefined, false);
  });

  it('does not replace an unreadable system version with a managed version', async () => {
    vi.stubEnv('VP_CLI_BIN', undefined);
    vi.spyOn(fs, 'existsSync').mockReturnValue(true);
    run.mockResolvedValue({
      exitCode: 0,
      stdout: Buffer.from(
        JSON.stringify({
          package_manager: {
            name: 'npm',
            version: 'unknown',
            source: 'system PATH',
            bin_paths: { npm: '/system/bin/npm' },
          },
        }),
      ),
      stderr: Buffer.alloc(0),
    });

    await expect(resolveCreatePackageManager(PackageManager.npm, 'latest')).rejects.toThrow(
      'Could not determine the system npm version and executable',
    );
    expect(download).not.toHaveBeenCalled();
    expect(run).toHaveBeenCalledWith(
      expect.objectContaining({
        command: path.join('/vp/bin', process.platform === 'win32' ? 'vp.exe' : 'vp'),
      }),
    );
  });
});
