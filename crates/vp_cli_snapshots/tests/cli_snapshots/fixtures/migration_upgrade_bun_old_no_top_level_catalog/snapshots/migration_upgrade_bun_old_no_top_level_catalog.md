# migration_upgrade_bun_old_no_top_level_catalog

Bun before 1.2.21 reads only `workspaces.catalog`, so a top-level `catalog` would fail to resolve.

## `vp migrate --no-interactive`

an empty-workspaces bun 1.2.20 project without a catalog upgrades

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

toolchain specs stay concrete and no top-level catalog is written

```
{
  "name": "migration-upgrade-bun-old-no-top-level-catalog",
  "workspaces": [],
  "devDependencies": {
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>",
    "vite-plus": "<version>"
  },
  "overrides": {
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>"
  },
  "devEngines": {
    "packageManager": {
      "name": "bun",
      "version": "<version>",
      "onFail": "download"
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
