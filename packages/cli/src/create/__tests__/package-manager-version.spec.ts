import fs from 'node:fs';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { PackageManager } from '../../types/index.js';
import { resolveCreatePackageManagerVersion } from '../package-manager-version.js';

vi.mock('../../../binding/index.js', () => ({
  getVpDirs: () => ({ bin: '/vite-plus/bin' }),
}));

const unpinned = { packageManager: undefined, packageManagerVersion: 'latest' };

describe('resolveCreatePackageManagerVersion', () => {
  beforeEach(() => {
    vi.stubEnv('npm_config_user_agent', 'npm/11.19.0 node/v26.8.1 darwin arm64');
    vi.stubEnv('VP_CLI_BIN', undefined);
    vi.spyOn(fs, 'existsSync').mockReturnValue(false);
  });

  afterEach(() => {
    vi.restoreAllMocks();
    vi.unstubAllEnvs();
  });

  it('uses the invoking npm version without a global CLI', () => {
    expect(resolveCreatePackageManagerVersion(PackageManager.npm, unpinned)).toBe('11.19.0');
  });

  it.each(['10.9.3', '12.2.1', '12.3.0-beta.1'])(
    'accepts the exact launcher version %s',
    (version) => {
      vi.stubEnv('npm_config_user_agent', `npm/${version} node/v26.8.1`);
      expect(resolveCreatePackageManagerVersion(PackageManager.npm, unpinned)).toBe(version);
    },
  );

  it.each(['latest', '^11.0.0', '11', '11.19', 'v11.19.0', '', '11.19.0/extra'])(
    'keeps the fallback for invalid launcher version %s',
    (version) => {
      vi.stubEnv('npm_config_user_agent', `npm/${version} node/v26.8.1`);
      expect(resolveCreatePackageManagerVersion(PackageManager.npm, unpinned)).toBe('latest');
    },
  );

  it.each([undefined, 'vp/1.4.0 npm/11.19.0', 'pnpm/12.11.2 npm/?', 'yarn/4.18.1'])(
    'keeps the fallback for another or missing launcher: %s',
    (agent) => {
      vi.stubEnv('npm_config_user_agent', agent);
      expect(resolveCreatePackageManagerVersion(PackageManager.npm, unpinned)).toBe('latest');
    },
  );

  it.each([PackageManager.pnpm, PackageManager.yarn, PackageManager.bun])(
    'does not apply the npm version to selected %s',
    (manager) => {
      expect(resolveCreatePackageManagerVersion(manager, unpinned)).toBe('latest');
    },
  );

  it.each(['10.9.3', '^12.0.0', 'latest'])(
    'preserves the detected npm requirement %s',
    (version) => {
      expect(
        resolveCreatePackageManagerVersion(PackageManager.npm, {
          packageManager: PackageManager.npm,
          packageManagerVersion: version,
        }),
      ).toBe(version);
    },
  );

  it('does not use a detected version from a different selected family', () => {
    expect(
      resolveCreatePackageManagerVersion(PackageManager.npm, {
        packageManager: PackageManager.pnpm,
        packageManagerVersion: '12.11.2',
      }),
    ).toBe('11.19.0');
  });

  it('leaves version resolution unchanged when launched by the global CLI', () => {
    vi.stubEnv('VP_CLI_BIN', '/custom/vp');
    expect(resolveCreatePackageManagerVersion(PackageManager.npm, unpinned)).toBe('latest');
    expect(fs.existsSync).not.toHaveBeenCalled();
  });

  it('leaves version resolution unchanged when the global resolver is installed', () => {
    vi.mocked(fs.existsSync).mockReturnValue(true);
    expect(resolveCreatePackageManagerVersion(PackageManager.npm, unpinned)).toBe('latest');
    expect(fs.existsSync).toHaveBeenCalledWith(
      path.join('/vite-plus/bin', process.platform === 'win32' ? 'vp.exe' : 'vp'),
    );
  });
});
