# command_update_catalog_protocol_yarn

## `vp migrate --no-interactive --no-hooks --package-manager yarn`

migrate pins the toolchain through the Yarn catalog

```
VITE+ - The Unified Toolchain for the Web

⚠ Vite+ does not currently support Yarn Plug'n'Play (PnP).

✔ Switched Yarn to node-modules mode

Formatting code...

Code formatted
◇ Migrated . to Vite+ <version>
• Node <version>  yarn <version>
✓ Dependencies installed in <duration>
• 1 config update applied
• Package manager settings configured
```

## `vpt print-file package.json`

the migrated project references the catalog

```
{
  "name": "command-update-catalog-protocol-yarn",
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "resolutions": {
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>"
  },
  "packageManager": "yarn@4.12.0"
}
```

## `vp up vite vite-plus`

#2309 Yarn variant: `yarn up` rewrites `catalog:` specs, so catalog-pinned names are skipped

```
warn: Skipped vite: the Yarn catalog pins its version, and `yarn up` would overwrite the `catalog:` reference. Edit the catalog entry in .yarnrc.yml, or run `vp migrate` when Vite+ manages the pin.
warn: Skipped vite-plus: the Yarn catalog pins its version, and `yarn up` would overwrite the `catalog:` reference. Edit the catalog entry in .yarnrc.yml, or run `vp migrate` when Vite+ manages the pin.
```

## `vpt print-file package.json`

`vite` and `vite-plus` stay `catalog:` instead of concrete ranges

```
{
  "name": "command-update-catalog-protocol-yarn",
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "resolutions": {
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>"
  },
  "packageManager": "yarn@4.12.0"
}
```

## `vpt print-file .yarnrc.yml`

the catalog keeps owning the resolved toolchain version

```
nodeLinker: node-modules
npmPreapprovedPackages:
  - vitest
  - "@vitest/*"
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vite-plus: <version>
```
