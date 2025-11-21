import type { DownloadPackageManagerResult } from '@voidzero-dev/vite-plus/binding';

import type { PackageManager } from './package.ts';

export interface WorkspacePackage {
  name: string;
  // The path of the package relative to the workspace root
  path: string;
  description?: string;
  version?: string;
  isTemplatePackage: boolean;
}

export interface WorkspaceInfo {
  rootDir: string;
  isMonorepo: boolean;
  // The scope of the monorepo, e.g. @my
  // This is used to determine the scope of the generated package
  // For example, if the monorepo scope is @my, then the generated package will be @my/my-package
  monorepoScope: string;
  // The patterns of the workspace packages
  // For example, ["apps/*", "packages/*", "services/*", "tools/*"]
  workspacePatterns: string[];
  // The parent directories of the generated package
  // For example, ["apps", "packages", "services", "tools"]
  parentDirs: string[];
  packageManager: PackageManager;
  packageManagerVersion: string;
  downloadPackageManager: DownloadPackageManagerResult;
  packages: WorkspacePackage[];
}

export interface WorkspaceInfoOptional extends Omit<
  WorkspaceInfo,
  'packageManager' | 'downloadPackageManager'
> {
  packageManager?: PackageManager;
}
