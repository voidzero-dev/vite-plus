/**
 * Supported package managers.
 *
 * Note: Aube is treated as pnpm-compatible for the parts of Vite+ that interact
 * with pnpm-style workspace YAML and manifest config (e.g. `aube-workspace.yaml`,
 * plus `package.json` `aube.*` / `pnpm.*` settings).
 */
export const PackageManager = {
  pnpm: 'pnpm',
  aube: 'aube',
  npm: 'npm',
  yarn: 'yarn',
  bun: 'bun',
} as const;
export type PackageManager = (typeof PackageManager)[keyof typeof PackageManager];

export const DependencyType = {
  dependencies: 'dependencies',
  devDependencies: 'devDependencies',
  peerDependencies: 'peerDependencies',
  optionalDependencies: 'optionalDependencies',
} as const;
export type DependencyType = (typeof DependencyType)[keyof typeof DependencyType];
