# migration_tsx_kept

tsx comes from a local stub package so the install stays offline

## `vpt json-edit package.json scripts.repl tsx`


## `vpt json-edit package.json scripts.calc 'tsx -e '\''console.log(1 + 1)'\'''`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

scripts vpx cannot run keep tsx, with a warning for each

```
VITE+ - The Unified Toolchain for the Web

● tsx scripts detected. Auto-migrating to vpx...
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 config update applied
• tsx scripts migrated to `vpx`
! Warnings:
  - Kept tsx in package.json scripts "calc": `vpx` does not run TypeScript passed to --eval or --print.
  - Kept tsx in package.json scripts "repl": `vpx` does not start a TypeScript REPL.
  - Kept the tsx devDependency in package.json: `scripts` in package.json still uses tsx.
```

## `vpt print-file package.json`

the other scripts run vpx, and the tsx devDependency stays

```
{
  "devDependencies": {
    "tsx": "file:.fixture/tsx",
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "name": "migration-tsx",
  "scripts": {
    "bench": "vpx ./scripts/bench.ts",
    "calc": "tsx -e 'console.log(1 + 1)'",
    "dev": "vpx --watch --clear-screen=false src/server.ts",
    "repl": "tsx",
    "seed": "cross-env NODE_ENV=development vpx scripts/seed.ts",
    "start": "concurrently \"vpx --watch src/server.ts\" \"vp dev\""
  },
  "type": "module"
}
```
