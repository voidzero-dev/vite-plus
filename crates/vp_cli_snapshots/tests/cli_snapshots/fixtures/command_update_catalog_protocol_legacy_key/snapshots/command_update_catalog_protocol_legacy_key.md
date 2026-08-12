# command_update_catalog_protocol_legacy_key

#2309 repair path. Migrate first so the catalog holds a real PINNED toolchain version (the reporter's shape), then downgrade the override key to the pre-fix bare spelling a project migrated by an older Vite+ still carries.

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

## `vpt replace-file-content pnpm-workspace.yaml vite@*: vite:`

rewind the override key to the pre-#2309 bare spelling (fails if migrate stopped writing the ranged key)


## `vpt print-file pnpm-workspace.yaml`

the pre-fix shape: bare override key over an exactly pinned catalog alias

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

## `vp up`

nothing to update, yet the bare key still resolves the catalog reference away

```
✓ Lockfile passes supply-chain policies (verified <duration> ago)
Already up to date

Done in <duration> using pnpm <version>
```

## `vpt print-file package.json`

`vite` lost `catalog:` for the pinned alias; `vite-plus` (no override) kept it

```
{
  "name": "command-update-catalog-protocol-legacy-key",
  "private": true,
  "devDependencies": {
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>",
    "vite-plus": "catalog:"
  },
  "packageManager": "pnpm@11.20.0"
}
```

## `vp migrate --no-interactive --no-hooks`

the bare key reads as pending, so one migrate repairs it

```
VITE+ - The Unified Toolchain for the Web

Formatting code...

Code formatted
◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
✓ Dependencies installed in <duration>
• Package manager settings configured
```

## `vpt print-file pnpm-workspace.yaml`

the bare key is replaced by the range-qualified one

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

update is now a no-op on the catalog reference

```
✓ Lockfile passes supply-chain policies (verified <duration> ago)
Already up to date

Done in <duration> using pnpm <version>
```

## `vpt print-file package.json`

`vite` stays `catalog:` across the update

```
{
  "name": "command-update-catalog-protocol-legacy-key",
  "private": true,
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "packageManager": "pnpm@11.20.0"
}
```
