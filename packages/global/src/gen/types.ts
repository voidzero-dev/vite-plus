import type { DownloadPackageManagerResult } from '@voidzero-dev/vite-plus/binding';

export const BuiltinTemplate = {
  generator: 'vite:generator',
  monorepo: 'vite:monorepo',
  application: 'vite:application',
  library: 'vite:library',
} as const;
export type BuiltinTemplate =
  (typeof BuiltinTemplate)[keyof typeof BuiltinTemplate];

export const TemplateType = {
  builtin: 'builtin',
  bingo: 'bingo',
  remote: 'remote',
} as const;
export type TemplateType = (typeof TemplateType)[keyof typeof TemplateType];

export interface TemplateInfo {
  command: string;
  args: string[];
  envs: NodeJS.ProcessEnv;
  type: TemplateType;
  // The parent directory of the generated package, only for monorepo
  // For example, "packages"
  parentDir?: string;
  interactive?: boolean;
}

export interface BuiltinTemplateInfo extends Omit<TemplateInfo, 'parentDir'> {
  packageName: string;
  targetDir: string;
}

export const PackageManager = {
  pnpm: 'pnpm',
  npm: 'npm',
  yarn: 'yarn',
} as const;
export type PackageManager =
  (typeof PackageManager)[keyof typeof PackageManager];

export const DependencyType = {
  dependencies: 'dependencies',
  devDependencies: 'devDependencies',
  peerDependencies: 'peerDependencies',
  optionalDependencies: 'optionalDependencies',
} as const;
export type DependencyType =
  (typeof DependencyType)[keyof typeof DependencyType];

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

export interface WorkspaceInfoOptional
  extends Omit<WorkspaceInfo, 'packageManager' | 'downloadPackageManager'> {
  packageManager?: PackageManager;
}

export interface WorkspacePackage {
  name: string;
  // The path of the package relative to the workspace root
  path: string;
  description?: string;
  version?: string;
  isTemplatePackage: boolean;
}

export interface ViteOptions {
  directory?: string;
  interactive: boolean;
  list: boolean;
  help: boolean;
}

export interface ExecutionResult {
  exitCode: number;
  projectDir?: string;
}
