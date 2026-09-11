# command_update_catalog_protocol_pnpm

## `vp migrate --no-interactive --no-hooks`

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

## `vpt print-file package.json`

the migrated project references the catalog

```
{
  "name": "command-update-catalog-protocol-pnpm",
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "packageManager": "pnpm@11.20.0"
}
```

## `vp up`

#2309: update must not resolve the catalog reference away

```
✓ Lockfile passes supply-chain policies (verified <duration> ago)
Already up to date

Done in <duration> using pnpm <version>
```

## `vpt print-file package.json`

`vite` stays `catalog:` instead of the concrete core alias

```
{
  "name": "command-update-catalog-protocol-pnpm",
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "packageManager": "pnpm@11.20.0"
}
```

## `vpt print-file pnpm-workspace.yaml`

the catalog keeps owning the resolved toolchain version

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
