import { PackageManager } from '../../types/package.ts';
import type { WorkspaceInfo } from '../../types/workspace.ts';

export function makeWorkspaceInfo(
  rootDir: string,
  overrides: Partial<WorkspaceInfo> = {},
): WorkspaceInfo {
  return {
    rootDir,
    packageManager: PackageManager.pnpm,
    packageManagerVersion: 'latest',
    isMonorepo: true,
    monorepoScope: '',
    workspacePatterns: [],
    parentDirs: [],
    packages: [],
    downloadPackageManager: {
      name: '',
      installDir: '',
      binPrefix: '',
      binPaths: {},
      packageName: '',
      version: '',
    },
    ...overrides,
  };
}
