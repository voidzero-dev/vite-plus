# review_only_rerun

Show a non-blocking review once in the final summary, and retain it on a no-op rerun without reconciling dependencies.

## `vpt write-file example.test.ts 'import { test } from '\''vitest'\'';
test('\''ui'\'', () => { const url = '\''http://localhost:51204/__vitest__/'\''; void url; });
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
  2:32 REVIEW [ui-token] Use the authenticated UI URL printed by Vitest, including its token.
    Docs: https://vitest.dev/guide/migration/#vitest-ui-requires-an-authenticated-url
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

Vitest v5: 1 review item

example.test.ts
  2:32 REVIEW [ui-token] Use the authenticated UI URL printed by Vitest, including its token.
    Docs: https://vitest.dev/guide/migration/#vitest-ui-requires-an-authenticated-url
This project is already using Vite+! Happy coding!
```
