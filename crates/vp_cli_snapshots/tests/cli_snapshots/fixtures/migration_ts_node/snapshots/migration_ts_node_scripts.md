# migration_ts_node_scripts

Local stub dependencies keep the runner migration independent of npm packages.

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

transpile-only commands and shebangs use vpx; preloads keep Node.js

```
VITE+ - The Unified Toolchain for the Web

● ts-node scripts detected. Auto-migrating supported commands to Vite+...
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 config update applied
• ts-node scripts migrated to Vite+
```

## `vpt print-file package.json`

```
{
  "name": "migration-ts-node",
  "type": "module",
  "scripts": {
    "seed": "vpx ./scripts/seed.ts",
    "loader": "node --import vite-plus/register ./scripts/seed.ts",
    "register": "node --import=vite-plus/register ./scripts/seed.ts",
    "env-loader": "NODE_OPTIONS='--import vite-plus/register' node ./scripts/seed.ts"
  },
  "devDependencies": {
    "typescript": "file:.fixture/typescript",
    "vite": "catalog:",
    "vite-plus": "catalog:"
  }
}
```

## `vpt print-file scripts/release.ts`

```
#!/usr/bin/env -S vpx
const version: string = '1.0.0';
console.log(version);
```

## `vp run seed`

the migrated script runs TypeScript without ts-node

```
$ vpx ./scripts/seed.ts ⊘ cache disabled
ready
```

## `vp run loader`

```
$ node --import vite-plus/register ./scripts/seed.ts ⊘ cache disabled
ready
```

## `vp run register`

the CommonJS registration option becomes the public ESM preload

```
$ node --import=vite-plus/register ./scripts/seed.ts ⊘ cache disabled
ready
```
