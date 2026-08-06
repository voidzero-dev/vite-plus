# migration_upgrade_monorepo_vitest_localized_pnpm

## `vp migrate --no-interactive`

existing Vite+ workspace packages should be reconciled

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

root should not gain a direct vitest

```
{
  "name": "migration-upgrade-monorepo-vitest-localized-pnpm",
  "private": true,
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
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

## `vpt print-file packages/app/package.json`

only the peer consumer should gain local vitest

```
{
  "name": "app",
  "devDependencies": {
    "@vitest/ui": "catalog:",
    "vite": "catalog:",
    "vite-plus": "catalog:",
    "vitest": "catalog:"
  }
}
```

## `vpt print-file pnpm-workspace.yaml`

shared vitest config should exist for the consuming package

```
packages:
  - packages/*
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vite-plus: <version>
  vitest: <version>
  '@vitest/ui': <version>
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
