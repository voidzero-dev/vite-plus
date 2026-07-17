# migration_standalone_pnpm

## `vp migrate --no-interactive --no-hooks --package-manager pnpm`

migration should work with pnpm, write overrides and peerDependencyRules to pnpm-workspace.yaml

```
VITE+ - The Unified Toolchain for the Web

Formatting code...

Code formatted
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
✓ Dependencies installed in <duration>
• 1 config update applied
```

## `vpt print-file package.json`

check package.json has no pnpm section

```
{
  "name": "migration-standalone-pnpm",
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "packageManager": "pnpm@10.33.2"
}
```

## `vpt print-file pnpm-workspace.yaml`

check pnpm-workspace.yaml has overrides, peerDependencyRules, and catalog

```
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vite-plus: <version>
overrides:
  vite: "catalog:"
peerDependencyRules:
  allowAny:
    - vite
  allowedVersions:
    vite: "*"
```
