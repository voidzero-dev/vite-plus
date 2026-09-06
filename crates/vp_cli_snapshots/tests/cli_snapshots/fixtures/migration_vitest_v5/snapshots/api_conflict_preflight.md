# api_conflict_preflight

## `vpt write-file vite.config.ts 'export default { test: { api: { port: 1 }, browser: { api: { port: 2 } } } };
'`


## `vp migrate --no-interactive`

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

Vitest v5: 1 review item (1 block dependency updates)

vite.config.ts
  1:55 BLOCK [api-conflict] Resolve conflicting test.api and test.browser.api values before upgrading.
Resolve the blocking Vitest v5 findings, then re-run `vp migrate`. No project files were changed.
```

## `vpt print-file package.json`

```
{
  "name": "migration-vitest-v5",
  "private": true,
  "type": "module",
  "packageManager": "pnpm@11.24.0",
  "scripts": {
    "test": "vitest list"
  },
  "devDependencies": {
    "vite": "^8.0.0",
    "vitest": "<version>"
  }
}
```
