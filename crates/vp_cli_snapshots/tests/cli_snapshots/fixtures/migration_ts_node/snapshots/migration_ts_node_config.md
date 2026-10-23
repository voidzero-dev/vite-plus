# migration_ts_node_config

## `vpt rm scripts/release.ts`


## `vpt write-file tsconfig.json '{"ts-node":{"transpileOnly":true}}
'`


## `vpt json-edit package.json scripts.seed 'ts-node ./scripts/seed.ts'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

tsconfig can explicitly disable ts-node type-checking

```
VITE+ - The Unified Toolchain for the Web

● ts-node scripts detected. Auto-migrating supported commands to vpx...
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 config update applied
• ts-node scripts migrated to `vpx`
```

## `vpt print-file package.json`

```
{
  "devDependencies": {
    "typescript": "file:.fixture/typescript",
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "name": "migration-ts-node",
  "scripts": {
    "loader": "vpx --tsconfig=tsconfig.json ./scripts/seed.ts",
    "seed": "vpx --tsconfig=tsconfig.json ./scripts/seed.ts"
  },
  "type": "module"
}
```

## `vp run seed`

```
$ vpx --tsconfig=tsconfig.json ./scripts/seed.ts ⊘ cache disabled
ready
```
