import fs from 'node:fs';
import path from 'node:path';

import { normalize } from 'verkit';

import { getVpDirs } from '../../binding/index.js';
import { PackageManager, type WorkspaceInfoOptional } from '../types/index.ts';

export function resolveCreatePackageManagerVersion(
  packageManager: PackageManager,
  workspace: Pick<WorkspaceInfoOptional, 'packageManager' | 'packageManagerVersion'>,
): string {
  if (packageManager === workspace.packageManager) {
    return workspace.packageManagerVersion;
  }

  const [launcher, version, extra] =
    process.env.npm_config_user_agent?.split(' ')[0]?.split('/') ?? [];
  if (
    packageManager !== PackageManager.npm ||
    launcher !== 'npm' ||
    extra !== undefined ||
    !version ||
    normalize(version) !== version ||
    process.env.VP_CLI_BIN
  ) {
    return 'latest';
  }

  const globalVp = path.join(getVpDirs().bin, process.platform === 'win32' ? 'vp.exe' : 'vp');
  if (fs.existsSync(globalVp)) {
    return 'latest';
  }

  // Without the global resolver, the next `npx vp run` uses the invoking npm.
  // Download that version too, so installation and the generated pin agree.
  return version;
}
