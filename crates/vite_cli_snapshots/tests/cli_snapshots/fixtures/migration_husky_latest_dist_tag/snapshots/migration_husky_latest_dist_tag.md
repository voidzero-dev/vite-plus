# migration_husky_latest_dist_tag

## `git init`


## `vp migrate --no-interactive`

should warn about uncoercible husky version

```
VITE+ - The Unified Toolchain for the Web

⚠ Could not determine husky version from "latest" — please specify a semver-compatible version (e.g., "^9.0.0") and re-run migration.
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 config update applied
```

## `vpt print-file package.json`

husky should still be in devDeps

```
{
  "name": "migration-husky-latest-dist-tag",
  "scripts": {
    "prepare": "husky"
  },
  "devDependencies": {
    "husky": "latest",
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

## `vpt print-file pnpm-workspace.yaml`

check pnpm-workspace.yaml has overrides and catalog

```
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vite-plus: <version>
overrides:
  vite: 'catalog:'
peerDependencyRules:
  allowAny:
    - vite
  allowedVersions:
    vite: '*'
```
