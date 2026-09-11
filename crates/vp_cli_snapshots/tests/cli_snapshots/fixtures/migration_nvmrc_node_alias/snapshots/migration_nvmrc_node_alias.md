# migration_nvmrc_node_alias

## `vp migrate --no-interactive`

'node' alias should be mapped to lts/* with an info message

```
VITE+ - The Unified Toolchain for the Web

"node" in .nvmrc is not a specific version; automatically mapping to "lts/*"
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 config updates applied
• Node version manager file migrated to .node-version
```

## `vpt print-file .node-version`

check node alias is mapped to lts/*

```
lts/*
```

## `vpt stat-file .nvmrc --assert-not file`

check .nvmrc is removed

```
.nvmrc: missing
```
