# command_update_catalog_protocol_legacy_key

#2309 repair path: a project migrated before the fix carries the bare `vite` override key, which `vp up` uses to clobber `catalog:`. One `vp migrate` re-keys it.

## `vp up`

the bare override key resolves the catalog reference away


## `vpt print-file package.json`

`vite` lost `catalog:`; `vite-plus` (no override) kept it

```
{
  "name": "command-update-catalog-protocol-legacy-key",
  "private": true,
  "devDependencies": {
    "vite": "npm:@voidzero-dev/vite-plus-core@^<version>",
    "vite-plus": "catalog:"
  },
  "packageManager": "pnpm@11.20.0"
}
```

## `vp migrate --no-interactive --no-hooks`

migrate repairs both the override key and the clobbered spec

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
