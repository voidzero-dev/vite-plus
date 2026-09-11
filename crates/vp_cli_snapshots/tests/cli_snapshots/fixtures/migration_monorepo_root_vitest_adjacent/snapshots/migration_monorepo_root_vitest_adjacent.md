# migration_monorepo_root_vitest_adjacent

## `vp migrate --no-interactive`

root with a vitest-adjacent dep but no direct vitest still gets a direct vitest pin

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 config updates applied
```

## `vpt print-file package.json`

vitest pinned at the root even though vite-plus is injected first

```
{
  "name": "migration-monorepo-root-vitest-adjacent",
  "scripts": {
    "test": "vp test",
    "prepare": "vp config"
  },
  "devDependencies": {
    "vite": "catalog:",
    "vitest-browser-svelte": "^2.1.0",
    "vite-plus": "catalog:",
    "vitest": "catalog:"
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
