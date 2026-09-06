# migration_nvmrc_lts

## `vp migrate --no-interactive`

migration should detect .nvmrc with lts alias and auto-migrate

```
VITE+ - The Unified Toolchain for the Web

Vitest v5: 1 review item

.nvmrc
  1:1 REVIEW [node-runtime] Resolve .nvmrc (lts/iron) and select Node ^22.18.0 || ^24.11.0 || >=26.0.0.
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 config updates applied
• Node version manager file migrated to .node-version
! Warnings:
  - Vitest v5: 1 review item

.node-version
  1:1 REVIEW [node-runtime] Resolve .node-version (lts/iron) and select Node ^22.18.0 || ^24.11.0 || >=26.0.0.
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
