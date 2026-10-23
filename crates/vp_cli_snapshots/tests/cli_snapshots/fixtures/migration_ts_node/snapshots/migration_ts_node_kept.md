# migration_ts_node_kept

## `vpt json-edit package.json scripts.checked 'ts-node ./scripts/seed.ts'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

commands that type-check keep ts-node and explain the difference

```
VITE+ - The Unified Toolchain for the Web

● ts-node scripts detected. Auto-migrating supported commands to vpx...
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 config update applied
• ts-node scripts migrated to `vpx`
! Warnings:
  - Kept ts-node in package.json scripts "checked": this ts-node command type-checks before execution; `vpx` does not type-check. Keep a separate type-check command before changing this script.
  - Kept the ts-node devDependency in package.json: `scripts` in package.json still uses ts-node.
```

## `vpt print-file package.json`

```
{
  "devDependencies": {
    "ts-node": "file:.fixture/ts-node",
    "typescript": "file:.fixture/typescript",
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "name": "migration-ts-node",
  "scripts": {
    "checked": "ts-node ./scripts/seed.ts",
    "loader": "vpx ./scripts/seed.ts",
    "seed": "vpx ./scripts/seed.ts"
  },
  "type": "module"
}
```
