# v4_reviews_do_not_repeat

Show execution-related v4 reviews in the first final summary, but not after upgrading to v5.

## `vpt write-file example.test.ts 'import { expect, test } from '\''vitest'\'';
test('\''poll'\'', async () => { await expect.poll(() => 42).toBe(42); });
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 files had imports rewritten
! Warnings:
  - Vitest v5: 1 review item

example.test.ts
  2:34 REVIEW [poll-timeout] Review the configured expect.poll timeout; v5 rejects assertions that finish after it.
    Docs: https://vitest.dev/guide/migration/#expect-poll-fails-when-it-times-out
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```
