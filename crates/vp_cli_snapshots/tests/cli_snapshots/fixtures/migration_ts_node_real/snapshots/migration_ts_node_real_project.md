# migration_ts_node_real_project

The local registry fetches pinned published ts-node 10.9.2 and TypeScript 5.9.3 on the first run to compare the real compilers; response bytes are cached for the run and installations remain case-local. Explicit transpile-only options override the selected config.

## `vp install --ignore-scripts`


## `vp run cli`

Before migration: the CLI --project selects the project JSX factory, even though that config disables transpile-only.

```
$ ts-node --transpileOnly --project ./config/tsconfig.json ./scripts/config.tsx ⊘ cache disabled
{"config":"project","tag":"result","value":"typescript"}
```

## `vp run register`

Before migration: TS_NODE_PROJECT and register/transpile-only select the same config and execution mode.

```
$ TS_NODE_PROJECT=./config/tsconfig.json node --require ts-node/register/transpile-only ./scripts/config.tsx ⊘ cache disabled
{"config":"project","tag":"result","value":"typescript"}
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
```

## `vpt print-file package.json`

```
{
  "name": "migration-ts-node-real",
  "packageManager": "pnpm@12.11.2",
  "scripts": {
    "cli": "vpx --tsconfig=./config/tsconfig.json ./scripts/config.tsx",
    "register": "TS_NODE_PROJECT=./config/tsconfig.json node --import vite-plus/register ./scripts/config.tsx"
  },
  "devDependencies": {
    "@types/node": "22.18.6",
    "typescript": "5.9.3",
    "vite-plus": "catalog:"
  }
}
```

## `vpt rm -rf node_modules`


## `vp install --ignore-scripts`


## `vpt stat-file node_modules/ts-node`

A clean install no longer contains ts-node.

```
node_modules/ts-node: missing
```

## `vp run cli`

After migration: vpx selects the same project JSX factory.

```
$ vpx --tsconfig=./config/tsconfig.json ./scripts/config.tsx ⊘ cache disabled
{"config":"project","tag":"result","value":"typescript"}
```

## `vp run register`

After migration: the public register produces the same output without ts-node.

```
$ TS_NODE_PROJECT=./config/tsconfig.json node --import vite-plus/register ./scripts/config.tsx ⊘ cache disabled
{"config":"project","tag":"result","value":"typescript"}
```
