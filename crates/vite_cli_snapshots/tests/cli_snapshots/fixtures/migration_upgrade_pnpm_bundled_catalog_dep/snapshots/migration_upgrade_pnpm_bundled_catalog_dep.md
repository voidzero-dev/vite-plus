# migration_upgrade_pnpm_bundled_catalog_dep

## `vp migrate --no-interactive`

existing Vite+ pnpm project declared bundled tools (oxlint/oxlint-tsgolint/oxfmt/tsdown) via catalog:

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  0.1.20 → <version>
    vite              → <version>
• Package manager settings configured
```

## `vpt print-file package.json`

the bundled tools are removed (vite-plus provides them) so no dangling catalog: references survive

```
{
  "name": "migration-upgrade-pnpm-bundled-catalog-dep",
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

## `vpt print-file pnpm-workspace.yaml`

the same bundled tools are dropped from the catalog, keeping package.json and the catalog consistent

```
packages:
  - .

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
