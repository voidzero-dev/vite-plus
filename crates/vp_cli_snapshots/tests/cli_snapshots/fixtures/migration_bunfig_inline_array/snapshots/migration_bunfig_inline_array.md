# migration_bunfig_inline_array

## `vp migrate --no-interactive`

migration preserves inline arrays in an existing bunfig.toml

```
VITE+ - The Unified Toolchain for the Web

Vitest v5: 1 review item

package.json
  1:1 REVIEW [configless-defaults] No test config exists. Vitest v5 clears mocks by default. A separate confirmed action can create compatibility config, but a new config can change config discovery and project structure.
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
