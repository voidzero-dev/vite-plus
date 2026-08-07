# migration_nvmrc

## `vp migrate --no-interactive`

migration should detect .nvmrc and auto-migrate to .node-version

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 config updates applied
• Node version manager file migrated to .node-version
```

## `vpt print-file .node-version`

check .node-version is created with v prefix stripped

```
25.8.2
```

## `vpt stat-file .nvmrc --assert-not file`

check .nvmrc is removed

```
.nvmrc: missing
```
