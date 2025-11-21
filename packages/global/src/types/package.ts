export const PackageManager = {
  pnpm: 'pnpm',
  npm: 'npm',
  yarn: 'yarn',
} as const;
export type PackageManager = (typeof PackageManager)[keyof typeof PackageManager];

export const DependencyType = {
  dependencies: 'dependencies',
  devDependencies: 'devDependencies',
  peerDependencies: 'peerDependencies',
  optionalDependencies: 'optionalDependencies',
} as const;
export type DependencyType = (typeof DependencyType)[keyof typeof DependencyType];
