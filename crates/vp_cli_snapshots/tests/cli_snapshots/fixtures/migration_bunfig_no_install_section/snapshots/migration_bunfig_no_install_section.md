# migration_bunfig_no_install_section

## `vp migrate --no-interactive`

migration preserves bunfig.toml without an install section

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  bun <version>
• 2 config updates applied
```

## `vpt print-file bunfig.toml`

check Bun configuration is unchanged

```
[run]
shell = "bun"
```
