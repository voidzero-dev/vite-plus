# mixed_project_assertions

Keep Node DOM matchers, migrate browser matchers, and retain review findings for shared transform inputs across repeated migrations.

## `vpt write-file vite.config.ts 'export default { test: { projects: [{ test: { name: '\''node'\'', include: ['\''node.test.ts'\'', '\''shared.test.ts'\''] } }, { test: { name: '\''browser'\'', browser: { enabled: true }, include: ['\''browser.test.ts'\'', '\''shared.test.ts'\''] } }] } };
'`


## `vpt write-file node.test.ts 'import { expect } from '\''vitest'\'';
expect(element).toHaveTextContent('\''partial'\'');
'`


## `vpt write-file browser.test.ts 'import { expect } from '\''vitest'\'';
await expect.element(element)['\''toHaveTextContent'\'']('\''partial'\'');
'`


## `vpt write-file shared.test.ts 'import { expect } from '\''vitest'\'';
expect(element).toHaveTextContent('\''partial'\'');
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

Vitest v5: 1 review item

shared.test.ts
  2:1 REVIEW [text-content-project] Resolve this assertion's test project: browser assertions need toMatchTextContent for v4 partial matching; keep Node jest-dom assertions unchanged.
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 4 files had imports rewritten
! Warnings:
  - Vitest v5: 1 review item

shared.test.ts
  2:1 REVIEW [text-content-project] Resolve this assertion's test project: browser assertions need toMatchTextContent for v4 partial matching; keep Node jest-dom assertions unchanged.
```

## `vpt print-file node.test.ts`

```
import { expect } from 'vite-plus/test';
expect(element).toHaveTextContent('partial');
```

## `vpt print-file browser.test.ts`

```
import { expect } from 'vite-plus/test';
await expect.element(element)["toMatchTextContent"]('partial');
```

## `vpt print-file shared.test.ts`

```
import { expect } from 'vite-plus/test';
expect(element).toHaveTextContent('partial');
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

Vitest v5: 1 review item

shared.test.ts
  2:1 REVIEW [text-content-project] Resolve this assertion's test project: browser assertions need toMatchTextContent for v4 partial matching; keep Node jest-dom assertions unchanged.
This project is already using Vite+! Happy coding!
```
