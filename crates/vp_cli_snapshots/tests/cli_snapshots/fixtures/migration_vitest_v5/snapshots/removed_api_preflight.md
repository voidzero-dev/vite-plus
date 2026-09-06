# removed_api_preflight

## `vpt write-file custom-runner.ts 'import { startTests } from '\''@vitest/runner'\'';
startTests([]);
'`


## `vp migrate --no-interactive`

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

Vitest v5: 1 review item (1 block dependency updates)

custom-runner.ts
  1:10 BLOCK [removed-api] Migrate startTests from @vitest/runner manually; no reviewed root v5 replacement exists.
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
