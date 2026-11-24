export const PackageManager = {
  pnpm: 'pnpm',
  npm: 'npm',
  yarn: 'yarn',
  bun: 'bun',
} as const;
export type PackageManager = (typeof PackageManager)[keyof typeof PackageManager];

export function isPackageManager(value: unknown): value is PackageManager {
  return typeof value === 'string' && Object.hasOwn(PackageManager, value);
}

export const DependencyType = {
  dependencies: 'dependencies',
  devDependencies: 'devDependencies',
  peerDependencies: 'peerDependencies',
  optionalDependencies: 'optionalDependencies',
} as const;
export type DependencyType = (typeof DependencyType)[keyof typeof DependencyType];
