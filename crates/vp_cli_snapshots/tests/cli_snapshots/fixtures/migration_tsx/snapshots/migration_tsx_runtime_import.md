# migration_tsx_runtime_import

## `vpt write-file src/runtime.ts 'const slash = /[//]/; import('\''tsx/esm/api'\'');
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

a runtime import after a regular expression keeps tsx

```
VITE+ - The Unified Toolchain for the Web

● tsx scripts detected. Auto-migrating to Vite+...
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 config update applied
• tsx scripts migrated to Vite+
! Warnings:
  - Kept the tsx devDependency in package.json: src/runtime.ts still uses tsx.
```

## `vpt print-file package.json`

scripts run vpx, but the runtime import still needs tsx

```
{
  "name": "migration-tsx",
  "type": "module",
  "scripts": {
    "dev": "vpx --watch --clear-screen=false src/server.ts",
    "seed": "cross-env NODE_ENV=development vpx scripts/seed.ts",
    "bench": "node --import vite-plus/register ./scripts/bench.ts",
    "start": "concurrently \"vpx --watch src/server.ts\" \"vp dev\""
  },
  "devDependencies": {
    "tsx": "file:.fixture/tsx",
    "vite": "catalog:",
    "vite-plus": "catalog:"
  }
}
```
