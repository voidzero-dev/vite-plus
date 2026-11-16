# migration_upgrade_bun_empty_workspaces_catalog

Bun resolves catalogs whenever the root package.json has a `workspaces` field, even with an empty package list (#3375).

## `vp migrate --no-interactive`

a single-package bun project with an empty workspaces.packages keeps its catalog

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  bun <version>
• Dependencies:
    vite-plus  0.1.20 → <version>
    vite              → <version>
• Package manager settings configured
```

## `vpt print-file package.json`

toolchain specs stay catalog: and the workspaces.catalog is rewritten in place

```
{
  "name": "migration-upgrade-bun-empty-workspaces-catalog",
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "overrides": {
    "vite": "catalog:"
  },
  "devEngines": {
    "packageManager": {
      "name": "bun",
      "version": "<version>",
      "onFail": "download"
    }
  },
  "workspaces": {
    "packages": [],
    "catalog": {
      "vite-plus": "<version>",
      "vite": "npm:@voidzero-dev/vite-plus-core@<version>"
    }
  }
}
```

## `vp migrate --no-interactive`

a second run converges to the already-migrated fast exit

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```
