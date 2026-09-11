# migration_nvmrc_lts

## `vp migrate --no-interactive`

migration should detect .nvmrc with lts alias and auto-migrate

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 config updates applied
• Node version manager file migrated to .node-version
```

## `vpt print-file .node-version`

check lts alias is preserved as-is

```
lts/iron
```

## `vpt stat-file .nvmrc --assert-not file`

check .nvmrc is removed

```
.nvmrc: missing
```
