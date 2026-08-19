# command_update_catalog_protocol_pnpm12

#2309 on pnpm 12. pnpm 12 fixes the clobber upstream (a bare override key no longer strips a `catalog:` importer spec), so this pins that the range-qualified key vite-plus writes is also correct there rather than only being a pnpm 9-11 workaround.

## `vp migrate --no-interactive --no-hooks --package-manager pnpm`

migrate pins the toolchain through the workspace catalog

```
VITE+ - The Unified Toolchain for the Web

Formatting code...

Code formatted
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
✓ Dependencies installed in <duration>
• 1 config update applied
```

## `vpt print-file pnpm-workspace.yaml`

the range-qualified override key is written on pnpm 12 too

```
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vite-plus: <version>
overrides:
  vite@*: "catalog:"
peerDependencyRules:
  allowAny:
    - vite
  allowedVersions:
    vite: "*"
```

## `vp up`

update must not resolve the catalog reference away (screen omitted: pnpm 12's update summary reports a package-count delta that churns with the bundled dependency graph)


## `vpt print-file package.json`

`vite` stays `catalog:`

```
{
  "name": "command-update-catalog-protocol-pnpm12",
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "packageManager": "pnpm@12.0.0-rc.3"
}
```
