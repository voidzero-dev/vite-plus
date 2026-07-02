import fs from 'node:fs';
import path from 'node:path';

import { readJsonFile, writeJsonFile } from '../utils/json.ts';

const VITE_PLUS_CORE_PACKAGE = '@voidzero-dev/vite-plus-core';

interface NpmLockPackage {
  name?: string;
  resolved?: string;
}

interface NpmPackageLock {
  packages?: Record<string, NpmLockPackage>;
}

function isViteInstallPath(packagePath: string): boolean {
  return packagePath === 'node_modules/vite' || packagePath.endsWith('/node_modules/vite');
}

function isVitePlusCorePackage(pkg: NpmLockPackage | undefined): boolean {
  return (
    pkg?.name === VITE_PLUS_CORE_PACKAGE ||
    pkg?.resolved?.includes('/@voidzero-dev/vite-plus-core/') === true
  );
}

interface MovedViteInstall {
  originalPath: string;
  backupPath: string;
}

// Dot-prefixed so npm treats the backup as an internal directory instead of an
// installed package while it sits inside node_modules during the reinstall.
const STALE_VITE_BACKUP_NAME = '.vite-plus-migrate-stale-vite';

function removeStaleInstalledVite(packagePath: string, moved: MovedViteInstall[]): boolean {
  const packageJsonPath = path.join(packagePath, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return false;
  }

  try {
    const pkg = readJsonFile(packageJsonPath) as { name?: string };
    if (pkg.name === VITE_PLUS_CORE_PACKAGE) {
      return false;
    }
  } catch {
    // A broken package directory also needs to be replaced by the reinstall.
  }

  // Move the stale install aside instead of deleting it, so a failed reinstall
  // can put the previously working Vite back rather than leaving the project
  // with no vite package at all.
  const backupPath = path.join(path.dirname(packagePath), STALE_VITE_BACKUP_NAME);
  try {
    fs.rmSync(backupPath, { recursive: true, force: true });
    fs.renameSync(packagePath, backupPath);
    moved.push({ originalPath: packagePath, backupPath });
  } catch {
    // Rename can fail on locked files (Windows). The stale install must still
    // go away or npm keeps resolving the real Vite instead of the alias.
    fs.rmSync(packagePath, { recursive: true, force: true });
  }
  return true;
}

export interface NpmViteAliasReinstallPreparation {
  changed: boolean;
  /** The reinstall succeeded: delete the moved-aside stale Vite installs. */
  commit: () => void;
  /** The reinstall failed: put the previously working Vite installs back. */
  restore: () => void;
}

/**
 * npm does not replace an already-installed package when its dependency changes
 * from `vite` to the `@voidzero-dev/vite-plus-core` npm alias. Even `npm
 * install --force` can exit successfully while retaining the real Vite package
 * and its stale package-lock entry. Move those stale Vite installs aside (and
 * drop their lock entries) before the migration's final install so npm resolves
 * the managed alias afresh; the caller commits or restores them depending on
 * whether the install succeeded, so a failed install does not strand the
 * project without any vite package.
 */
export function prepareNpmViteAliasReinstall(
  rootDir: string,
  projectPaths: string[] = [rootDir],
): NpmViteAliasReinstallPreparation {
  const packageLockPath = path.join(rootDir, 'package-lock.json');
  const moved: MovedViteInstall[] = [];
  let changed = false;

  if (fs.existsSync(packageLockPath)) {
    try {
      const packageLock = readJsonFile(packageLockPath) as NpmPackageLock;
      let lockChanged = false;

      for (const [packagePath, pkg] of Object.entries(packageLock.packages ?? {})) {
        if (!isViteInstallPath(packagePath)) {
          continue;
        }

        const installPath = path.resolve(rootDir, packagePath);
        const relativeInstallPath = path.relative(rootDir, installPath);
        if (relativeInstallPath.startsWith('..') || path.isAbsolute(relativeInstallPath)) {
          continue;
        }

        if (!isVitePlusCorePackage(pkg)) {
          delete packageLock.packages?.[packagePath];
          lockChanged = true;
          removeStaleInstalledVite(installPath, moved);
        } else {
          changed = removeStaleInstalledVite(installPath, moved) || changed;
        }
      }

      if (lockChanged) {
        writeJsonFile(packageLockPath, packageLock as unknown as Record<string, unknown>);
        changed = true;
      }
    } catch {
      // A malformed, truncated, or merge-conflicted package-lock.json cannot be
      // safely rewritten. Skip lockfile reconciliation instead of aborting the
      // migration mid-write; the final `npm install --force` regenerates it.
    }
  }

  // Also handle installs without a lockfile and workspace-local copies that do
  // not have their own package-lock entry.
  for (const projectPath of projectPaths) {
    changed =
      removeStaleInstalledVite(path.join(projectPath, 'node_modules', 'vite'), moved) || changed;
  }

  return {
    changed,
    commit: () => {
      for (const { backupPath } of moved) {
        try {
          fs.rmSync(backupPath, { recursive: true, force: true });
        } catch {
          // A leftover dot-directory in node_modules is harmless.
        }
      }
    },
    restore: () => {
      for (const { originalPath, backupPath } of moved) {
        try {
          // Whatever a failed install left behind is suspect; prefer the
          // known-good previous install.
          fs.rmSync(originalPath, { recursive: true, force: true });
          fs.renameSync(backupPath, originalPath);
        } catch {
          // Best effort: the follow-up `vp install` reinstalls from scratch.
        }
      }
    },
  };
}
