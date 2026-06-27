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

function removeStaleInstalledVite(packagePath: string): boolean {
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

  fs.rmSync(packagePath, { recursive: true, force: true });
  return true;
}

/**
 * npm does not replace an already-installed package when its dependency changes
 * from `vite` to the `@voidzero-dev/vite-plus-core` npm alias. Even `npm
 * install --force` can exit successfully while retaining the real Vite package
 * and its stale package-lock entry. Remove only those stale Vite entries before
 * the migration's final install so npm resolves the managed alias afresh.
 */
export function prepareNpmViteAliasReinstall(
  rootDir: string,
  projectPaths: string[] = [rootDir],
): boolean {
  const packageLockPath = path.join(rootDir, 'package-lock.json');
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
          removeStaleInstalledVite(installPath);
        } else {
          changed = removeStaleInstalledVite(installPath) || changed;
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
    changed = removeStaleInstalledVite(path.join(projectPath, 'node_modules', 'vite')) || changed;
  }

  return changed;
}
