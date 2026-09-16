# review_only_rerun

Show a non-blocking review once in the final summary, and retain it on a no-op rerun without reconciling dependencies.

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

Vitest v5: 1 review item

example.test.ts
  2:34 REVIEW [poll-timeout] Review the configured expect.poll timeout; v5 rejects assertions that finish after it.
    Docs: https://vitest.dev/guide/migration/#expect-poll-fails-when-it-times-out
This project is already using Vite+! Happy coding!
```
