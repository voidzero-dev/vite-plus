# command_update_catalog_protocol_legacy_key

#2309 repair path. Migrate first so the catalog holds a real PINNED toolchain version (the reporter's shape), then downgrade the override key to the pre-fix bare spelling a project migrated by an older Vite+ still carries.

## `vp migrate --no-interactive --no-hooks`

migrate pins the toolchain through the workspace catalog

```
VITE+ - The Unified Toolchain for the Web

●  Formatting code...
●  Code formatted
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
[WARN] 24 deprecated subdependencies found: @yuku-codegen/binding-android-arm64@0.9.5, @yuku-codegen/binding-darwin-arm64@0.9.5, @yuku-codegen/binding-darwin-x64@0.9.5, @yuku-codegen/binding-freebsd-x64@0.9.5, @yuku-codegen/binding-linux-arm-gnu@0.9.5, @yuku-codegen/binding-linux-arm-musl@0.9.5, @yuku-codegen/binding-linux-arm64-gnu@0.9.5, @yuku-codegen/binding-linux-arm64-musl@0.9.5, @yuku-codegen/binding-linux-x64-gnu@0.9.5, @yuku-codegen/binding-linux-x64-musl@0.9.5, @yuku-codegen/binding-win32-arm64@0.9.5, @yuku-codegen/binding-win32-x64@0.9.5, @yuku-parser/binding-android-arm64@0.9.5, @yuku-parser/binding-darwin-arm64@0.9.5, @yuku-parser/binding-darwin-x64@0.9.5, @yuku-parser/binding-freebsd-x64@0.9.5, @yuku-parser/binding-linux-arm-gnu@0.9.5, @yuku-parser/binding-linux-arm-musl@0.9.5, @yuku-parser/binding-linux-arm64-gnu@0.9.5, @yuku-parser/binding-linux-arm64-musl@0.9.5, @yuku-parser/binding-linux-x64-gnu@0.9.5, @yuku-parser/binding-linux-x64-musl@0.9.5, @yuku-parser/binding-win32-arm64@0.9.5, @yuku-parser/binding-win32-x64@0.9.5
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

●  Formatting code...
●  Code formatted
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
[WARN] 24 deprecated subdependencies found: @yuku-codegen/binding-android-arm64@0.9.5, @yuku-codegen/binding-darwin-arm64@0.9.5, @yuku-codegen/binding-darwin-x64@0.9.5, @yuku-codegen/binding-freebsd-x64@0.9.5, @yuku-codegen/binding-linux-arm-gnu@0.9.5, @yuku-codegen/binding-linux-arm-musl@0.9.5, @yuku-codegen/binding-linux-arm64-gnu@0.9.5, @yuku-codegen/binding-linux-arm64-musl@0.9.5, @yuku-codegen/binding-linux-x64-gnu@0.9.5, @yuku-codegen/binding-linux-x64-musl@0.9.5, @yuku-codegen/binding-win32-arm64@0.9.5, @yuku-codegen/binding-win32-x64@0.9.5, @yuku-parser/binding-android-arm64@0.9.5, @yuku-parser/binding-darwin-arm64@0.9.5, @yuku-parser/binding-darwin-x64@0.9.5, @yuku-parser/binding-freebsd-x64@0.9.5, @yuku-parser/binding-linux-arm-gnu@0.9.5, @yuku-parser/binding-linux-arm-musl@0.9.5, @yuku-parser/binding-linux-arm64-gnu@0.9.5, @yuku-parser/binding-linux-arm64-musl@0.9.5, @yuku-parser/binding-linux-x64-gnu@0.9.5, @yuku-parser/binding-linux-x64-musl@0.9.5, @yuku-parser/binding-win32-arm64@0.9.5, @yuku-parser/binding-win32-x64@0.9.5
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
