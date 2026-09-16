# removed_api_preflight

Show blockers and accompanying reviews once before stopping without dependency changes.

## `vpt write-file custom-runner.ts 'import { startTests } from '\''@vitest/runner'\'';
startTests([]);
'`


## `vpt write-file review.test.ts 'import { expect } from '\''vitest'\'';
await expect.poll(() => 42).toBe(42);
'`


## `vp migrate --no-interactive`

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

Vitest v5: 2 review items (1 block dependency updates)

custom-runner.ts
  1:10 BLOCK [removed-api] Migrate startTests from @vitest/runner manually; no reviewed root v5 replacement exists.
    Docs: https://vitest.dev/guide/migration/#removed-deprecated-entrypoints

review.test.ts
  2:7 REVIEW [poll-timeout] Review the configured expect.poll timeout; v5 rejects assertions that finish after it.
    Docs: https://vitest.dev/guide/migration/#expect-poll-fails-when-it-times-out
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
