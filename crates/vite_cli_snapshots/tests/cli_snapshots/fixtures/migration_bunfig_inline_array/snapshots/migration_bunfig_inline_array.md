# migration_bunfig_inline_array

## `vp migrate --no-interactive`

migration preserves inline arrays in an existing bunfig.toml

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  bun <version>
• 2 config updates applied
```

## `vpt print-file bunfig.toml`

check Bun configuration is unchanged

```
[install]
minimumReleaseAge = 259200
minimumReleaseAgeExcludes = ["@zerobyte/*"]
```
