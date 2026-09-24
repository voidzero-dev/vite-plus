import fs from 'node:fs';
import path from 'node:path';

import semver from 'semver';

import { getVpDirs } from '../../binding/index.js';
import type { PackageManager, WorkspaceInfo } from '../types/index.ts';
import { runCommandSilently } from '../utils/command.ts';
import { downloadPackageManager } from '../utils/prompts.ts';

interface CurrentPackageManager {
  name: string;
  version: string;
  source: string;
  bin_paths: Record<string, string>;
}

export async function resolveCreatePackageManager(
  packageManager: PackageManager,
  version: string,
  interactive?: boolean,
  silent = false,
): Promise<WorkspaceInfo['downloadPackageManager']> {
  const globalVp =
    process.env.VP_CLI_BIN ??
    path.join(getVpDirs().bin, process.platform === 'win32' ? 'vp.exe' : 'vp');
  // The local package can be used without a global installation. In that case
  // there is no global environment selection to honor.
  if (!process.env.VP_CLI_BIN && !fs.existsSync(globalVp)) {
    return downloadPackageManager(packageManager, version, interactive, silent);
  }
  // Use the same mode and system-tool lookup as the subsequent `vp install`.
  // A system-first mode can still fall back to managed when no system tool exists.
  const result = await runCommandSilently({
    command: globalVp,
    args: ['env', 'current', packageManager, '--json'],
    cwd: process.cwd(),
    envs: process.env,
  });
  if (result.exitCode !== 0) {
    throw new Error(`Failed to resolve ${packageManager}: ${result.stderr.toString().trim()}`);
  }
  const current = (
    JSON.parse(result.stdout.toString()) as {
      package_manager?: CurrentPackageManager;
    }
  ).package_manager;
  if (current?.source === 'system PATH') {
    const binPath = current.bin_paths[packageManager];
    if (current.name !== packageManager || !semver.valid(current.version) || !binPath) {
      throw new Error(`Could not determine the system ${packageManager} version and executable`);
    }
    return {
      name: packageManager,
      version: current.version,
      binPrefix: path.dirname(binPath),
    };
  }

  return downloadPackageManager(packageManager, version, interactive, silent);
}
