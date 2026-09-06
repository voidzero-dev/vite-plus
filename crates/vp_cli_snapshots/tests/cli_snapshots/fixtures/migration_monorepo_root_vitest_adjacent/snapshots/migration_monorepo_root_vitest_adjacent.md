# migration_monorepo_root_vitest_adjacent

## `vpt write-file node_modules/vitest/package.json '{"name":"vitest","version":"4.1.11"}'`

record the original runner version without adding a direct dependency


## `vp migrate --no-interactive`

root with a vitest-adjacent dep but no direct vitest still gets a direct vitest pin

```
VITE+ - The Unified Toolchain for the Web

Vitest v5: 1 review item

package.json
  1:1 REVIEW [configless-defaults] No test config exists. Vitest v5 clears mocks by default and uses exact browser locators. A separate confirmed action can create compatibility config, but a new config can change config discovery and project structure.
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
