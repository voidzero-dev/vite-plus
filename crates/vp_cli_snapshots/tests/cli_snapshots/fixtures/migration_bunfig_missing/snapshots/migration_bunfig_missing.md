# migration_bunfig_missing

## `vp migrate --no-interactive`

migration does not create bunfig.toml

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  bun <version>
• 2 config updates applied
```

## `vpt stat-file bunfig.toml --assert-not file`

check Bun configuration remains absent

```
bunfig.toml: missing
```
