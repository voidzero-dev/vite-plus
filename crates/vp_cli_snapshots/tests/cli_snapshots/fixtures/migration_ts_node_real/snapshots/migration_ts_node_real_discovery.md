# migration_ts_node_real_discovery

Real ts-node searches from the entry directory for its CLI and from the working directory for its Node.js preload. Different JSX factories make that distinction visible.

## `vpt json-edit package.json scripts.cli 'ts-node ./scripts/config.tsx'`


## `vpt json-edit package.json scripts.register 'node --require ts-node/register ./scripts/config.tsx'`


## `vp install --ignore-scripts`


## `vp run cli`

Before migration: CLI discovery uses scripts/tsconfig.json.

```
$ ts-node ./scripts/config.tsx ⊘ cache disabled
{"config":"entry","tag":"result","value":"typescript"}
```

## `vp run register`

Before migration: preload discovery uses the root tsconfig.json.

```
$ node --require ts-node/register ./scripts/config.tsx ⊘ cache disabled
{"config":"cwd","tag":"result","value":"typescript"}
```

## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

● ts-node scripts detected. Auto-migrating supported commands to Vite+...
◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  latest → <version>
    vite              → <version>
• ts-node scripts migrated to Vite+
• Package manager settings configured
! Warnings:
  - Kept ts-node in package.json scripts "register": the Node.js preload needs the ts-node config pinned explicitly; set TS_NODE_PROJECT to its file path before migrating.
  - Kept the ts-node devDependency in package.json: `scripts` in package.json still uses ts-node.
```

## `vpt print-file package.json`

The CLI pins its discovered config. The preload and ts-node dependency stay because its config was not explicitly selected.

```
{
  "devDependencies": {
    "@types/node": "22.18.6",
    "ts-node": "10.9.2",
    "typescript": "5.9.3",
    "vite-plus": "catalog:"
  },
  "name": "migration-ts-node-real",
  "packageManager": "pnpm@12.11.2",
  "scripts": {
    "cli": "vpx --tsconfig=scripts/tsconfig.json ./scripts/config.tsx",
    "register": "node --require ts-node/register ./scripts/config.tsx"
  }
}
```

## `vpt rm -rf node_modules`


## `vp install --ignore-scripts`


## `vp run cli`

After migration: vpx still uses the entry-directory factory.

```
$ vpx --tsconfig=scripts/tsconfig.json ./scripts/config.tsx ⊘ cache disabled
{"config":"entry","tag":"result","value":"typescript"}
```

## `vp run register`

After migration: the retained ts-node preload still uses the working-directory factory.

```
$ node --require ts-node/register ./scripts/config.tsx ⊘ cache disabled
{"config":"cwd","tag":"result","value":"typescript"}
```
