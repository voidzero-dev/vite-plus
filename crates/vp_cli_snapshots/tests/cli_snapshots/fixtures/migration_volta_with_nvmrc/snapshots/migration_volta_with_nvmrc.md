# migration_volta_with_nvmrc

## `vp migrate --no-interactive`

an existing .nvmrc is kept and volta.node is not converted

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 config updates applied
```

## `vpt stat-file .node-version --assert-not file`

no .node-version is written to shadow .nvmrc

```
.node-version: missing
```

## `vpt print-file .nvmrc`

.nvmrc is left untouched

```
<version>
```

## `vpt print-file package.json`

volta field must remain intact

```
{
  "name": "migration-volta-with-nvmrc",
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "volta": {
    "node": "18.0.0",
    "npm": "9.0.0"
  },
  "scripts": {
    "prepare": "vp config"
  }
}
```
