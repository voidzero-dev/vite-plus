# migration_oxnode_kept

## `vpt json-edit package.json scripts.node-watch 'oxnode --watch ./scripts/seed.ts'`


## `vpt write-file scripts/importer.ts 'import '\''@oxc-node/core/register'\'';
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

Node watch mode and remaining loader imports keep the dependencies

```
VITE+ - The Unified Toolchain for the Web

● oxnode scripts detected. Auto-migrating to Vite+...
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 config update applied
• oxnode scripts migrated to Vite+
! Warnings:
  - Kept oxnode in package.json scripts "node-watch": oxnode --watch starts the Node.js watcher, while `vpx --watch` uses its own watcher; use oxnode watch to migrate it.
  - Kept the @oxc-node/cli / @oxc-node/core devDependency in package.json: `scripts` in package.json still uses oxnode.
```

## `vpt print-file package.json`

```
{
  "devDependencies": {
    "@oxc-node/cli": "file:.fixture/oxc-cli",
    "@oxc-node/core": "file:.fixture/oxc-core",
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "name": "migration-oxnode",
  "scripts": {
    "dev": "vpx --watch --clear-screen=false ./scripts/seed.ts",
    "env-loader": "NODE_OPTIONS='--import vite-plus/register' node ./scripts/seed.ts",
    "loader": "node --import vite-plus/register ./scripts/seed.ts",
    "loader-eval": "node --import=vite-plus/register --eval \"import('./scripts/seed.ts')\"",
    "loader-test": "node --import vite-plus/register --test --test-reporter=dot ./scripts/seed.ts",
    "node-watch": "oxnode --watch ./scripts/seed.ts",
    "seed": "vpx -C dev ./scripts/seed.ts"
  },
  "type": "module"
}
```
