# migration_from_tsup_nested_runner

## `vpt chmod +x tsdown-migrate-stub.mjs`


## `vpt json-edit package.json scripts.build 'concurrently "tsup --watch --config=tsup.config.ts" "tsc --watch"'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

a nested tsdown command should migrate to vp pack

```
VITE+ - The Unified Toolchain for the Web

tsup configuration detected. Auto-migrating to tsdown...
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 config updates applied, 1 file had imports rewritten
• tsup config migrated to tsdown (`vp pack`)
! Warnings:
  - tsdown-migrate: The splitting option is currently unsupported in tsdown. Code splitting is always enabled and cannot be disabled.
→ Manual follow-up:
  - Please manually merge tsdown.config.ts into vite.config.ts, see https://viteplus.dev/guide/migrate#tsdown
```

## `vpt print-file package.json`

the quoted runner command uses vp pack

```
{
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "name": "migration-from-tsup",
  "scripts": {
    "build": "concurrently \"vp pack --watch\" \"tsc --watch\""
  },
  "devEngines": {
    "packageManager": {
      "name": "pnpm",
      "version": "<version>",
      "onFail": "download"
    }
  }
}
```
