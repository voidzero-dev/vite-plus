# migration_upgrade_bun_catalog

## `vp migrate --no-interactive`

existing bun Vite+ catalog WORKSPACE gains a direct vite edge for bun#8406 (bun resolves catalog: only inside a workspace)

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  bun <version>
• Dependencies:
    vite-plus  0.1.20 → <version>
    vite              → <version>
• Package manager settings configured
```

## `vp migrate --no-interactive`

the second migration finds the workspace up to date

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `vpt print-file package.json`

the direct Vite dependency and its override use the catalog, and the PostCSS pin survives both runs

```
{
  "name": "migration-upgrade-bun-catalog",
  "workspaces": [
    "packages/*"
  ],
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "overrides": {
    "vite": {
      ".": "catalog:",
      "postcss": "8.5.32"
    }
  },
  "devEngines": {
    "packageManager": {
      "name": "bun",
      "version": "<version>",
      "onFail": "download"
    }
  },
  "catalog": {
    "vite-plus": "<version>",
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>"
  }
}
```
