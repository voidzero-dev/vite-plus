# migration_tsx_already_vite_plus

On a project already using Vite+, the tsx migration belongs to `vp migrate --full`.

## `vpt json-edit package.json devDependencies.vite-plus latest`


## `vp migrate --no-interactive`

a plain upgrade leaves tsx scripts to --full

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  latest → <version>
    vite              → <version>
• Package manager settings configured
• Skipped editor, hooks, and lint setup. Run `vp migrate --full` to apply them.
```

## `vp migrate --no-interactive --full --no-hooks --no-agent --no-editor`

--full migrates tsx scripts to vpx

```
VITE+ - The Unified Toolchain for the Web

▲ No package manager is declared in package.json; using pnpm for this migration without adding a pin. Run `vp env pin` to declare it explicitly.
● tsx scripts detected. Auto-migrating to vpx...
◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite   → <version>
• tsx scripts migrated to `vpx`
```

## `vpt print-file package.json`

scripts run vpx, and the tsx devDependency is gone

```
{
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "name": "migration-tsx",
  "scripts": {
    "bench": "vpx ./scripts/bench.ts",
    "dev": "vpx --watch --clear-screen=false src/server.ts",
    "seed": "cross-env NODE_ENV=development vpx scripts/seed.ts",
    "start": "concurrently \"vpx --watch src/server.ts\" \"vp dev\""
  },
  "type": "module"
}
```
