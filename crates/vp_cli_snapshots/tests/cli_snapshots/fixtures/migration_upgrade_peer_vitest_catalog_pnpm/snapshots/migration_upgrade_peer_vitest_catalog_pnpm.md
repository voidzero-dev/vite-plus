# migration_upgrade_peer_vitest_catalog_pnpm

## `vp migrate --no-interactive`

peer catalog must resolve before managed Vitest catalogs are pruned

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  latest → <version>
    vite              → <version>
• Package manager settings configured
```

## `vpt print-file package.json`

peer uses its resolved public range without gaining direct Vitest

```
{
  "name": "migration-upgrade-peer-vitest-catalog-pnpm",
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "peerDependencies": {
    "vitest": "^4.0.0"
  },
  "devEngines": {
    "packageManager": {
      "name": "pnpm",
      "version": "<version>",
      "onFail": "download"
    }
  }
}
```

## `vpt print-file pnpm-workspace.yaml`

unreferenced managed Vitest catalog is removed

```
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vite-plus: <version>
catalogs:
  test: {}
overrides:
  vite: 'catalog:'
peerDependencyRules:
  allowAny:
    - vite
  allowedVersions:
    vite: '*'
```

## `vp migrate --no-interactive`

repaired project should no longer be pending

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```
