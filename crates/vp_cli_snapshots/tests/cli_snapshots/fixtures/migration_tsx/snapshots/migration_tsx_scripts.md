# migration_tsx_scripts

tsx comes from a local stub package so the install stays offline

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

tsx scripts should migrate to vpx automatically

```
VITE+ - The Unified Toolchain for the Web

● tsx scripts detected. Auto-migrating to vpx...
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 config update applied
• tsx scripts migrated to `vpx`
```

## `vpt print-file package.json`

scripts run vpx, and the tsx devDependency is gone

```
{
  "name": "migration-tsx",
  "type": "module",
  "scripts": {
    "dev": "vpx --watch --clear-screen=false src/server.ts",
    "seed": "cross-env NODE_ENV=development vpx scripts/seed.ts",
    "bench": "vpx ./scripts/bench.ts",
    "start": "concurrently \"vpx --watch src/server.ts\" \"vp dev\""
  },
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  }
}
```

## `vpt print-file scripts/release.ts`

the shebang runs vpx

```
#!/usr/bin/env vpx
const version: string = '1.0.0';
console.log(version);
```
