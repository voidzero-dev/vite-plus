# migration_upgrade_pkg_pr_new_pnpm

## `vp migrate --no-interactive`

bridge commit builds upgrade like an ordinary npm version

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus            0.1.20 → <version>
    vite                        → <version>
    vitest               0.1.20 → <version>
    @vitest/coverage-v8  4.1.6  → <version>
• Package manager settings configured
```

## `vpt print-file package.json`

direct dependencies use catalogs aligned to the bridge build

```
{
  "name": "migration-upgrade-pkg-pr-new-pnpm",
  "devDependencies": {
    "@vitest/coverage-v8": "catalog:",
    "vite": "catalog:",
    "vite-plus": "catalog:",
    "vitest": "catalog:"
  },
  "packageManager": "pnpm@10.33.2"
}
```

## `vpt print-file pnpm-workspace.yaml`

the catalog holds the immutable commit version

```
packages:
  - .

blockExoticSubdeps: true

catalog:
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vite-plus: <version>
  vitest: <version>
  '@vitest/coverage-v8': <version>

overrides:
  vite: 'catalog:'
  vitest: 'catalog:'

peerDependencyRules:
  allowAny:
    - vite
    - vitest
  allowedVersions:
    vite: '*'
    vitest: '*'
```

## `vp migrate --no-interactive`

bridge commit migration is idempotent

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `vpt print-file package.json`

rerun leaves package.json unchanged

```
{
  "name": "migration-upgrade-pkg-pr-new-pnpm",
  "devDependencies": {
    "@vitest/coverage-v8": "catalog:",
    "vite": "catalog:",
    "vite-plus": "catalog:",
    "vitest": "catalog:"
  },
  "packageManager": "pnpm@10.33.2"
}
```

## `vpt print-file pnpm-workspace.yaml`

rerun leaves the catalog unchanged

```
packages:
  - .

blockExoticSubdeps: true

catalog:
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vite-plus: <version>
  vitest: <version>
  '@vitest/coverage-v8': <version>

overrides:
  vite: 'catalog:'
  vitest: 'catalog:'

peerDependencyRules:
  allowAny:
    - vite
    - vitest
  allowedVersions:
    vite: '*'
    vitest: '*'
```
