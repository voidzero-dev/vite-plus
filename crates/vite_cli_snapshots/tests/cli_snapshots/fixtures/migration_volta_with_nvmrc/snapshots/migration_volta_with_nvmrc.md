# migration_volta_with_nvmrc

## `vp migrate --no-interactive`

.nvmrc should take priority over volta.node

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 config updates applied
• Node version manager file migrated to .node-version
→ Manual follow-up:
  - Remove the "volta" field from package.json
```

## `vpt print-file .node-version`

check .node-version comes from .nvmrc (v20.19.0), not volta.node (18.0.0)

```
20.19.0
```

## `vpt stat-file .nvmrc --assert-not file`

check .nvmrc is removed

```
.nvmrc: missing
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
  "devEngines": {
    "packageManager": {
      "name": "pnpm",
      "version": "<version>",
      "onFail": "download"
    }
  },
  "scripts": {
    "prepare": "vp config"
  }
}
```
