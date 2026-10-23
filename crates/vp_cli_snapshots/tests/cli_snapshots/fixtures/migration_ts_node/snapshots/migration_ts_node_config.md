# migration_ts_node_config

## `vpt rm scripts/release.ts`


## `vpt write-file tsconfig.json '{"ts-node":{"transpileOnly":true}}
'`


## `vpt json-edit package.json scripts.seed 'ts-node ./scripts/seed.ts'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

the runner pins its config; Node preloads keep ts-node until TS_NODE_PROJECT names that file

```
VITE+ - The Unified Toolchain for the Web

● ts-node scripts detected. Auto-migrating supported commands to Vite+...
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 config update applied
• ts-node scripts migrated to Vite+
! Warnings:
  - Kept ts-node in package.json scripts "env-loader": the Node.js preload needs the ts-node config pinned explicitly; set TS_NODE_PROJECT to its file path before migrating.
  - Kept ts-node in package.json scripts "loader": the Node.js preload needs the ts-node config pinned explicitly; set TS_NODE_PROJECT to its file path before migrating.
  - Kept ts-node in package.json scripts "register": the Node.js preload needs the ts-node config pinned explicitly; set TS_NODE_PROJECT to its file path before migrating.
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
    "env-loader": "NODE_OPTIONS='--require ts-node/register/transpile-only' node ./scripts/seed.ts",
    "loader": "node --loader ts-node/esm/transpile-only ./scripts/seed.ts",
    "register": "node --require=ts-node/register/transpile-only ./scripts/seed.ts",
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
