# unresolved_in_source_test_scopes

Keep an ownership review for dynamic includeSource patterns without suppressing normal test migration.

## `vpt rm example.test.ts`


## `vpt write-file vite.config.ts 'export default { test: { globals: true, includeSource: process.env.TEST_SOURCES?.split('\'','\'') ?? ['\''src/*.js'\''] } };
'`


## `vpt write-file src/add.js 'export const add = (a, b) => a + b;
if (import.meta.vitest) { test.sequential('\''in-source'\'', () => { expect(add(2, 3)).toBe(5); }); }
'`


## `vpt write-file control.test.js 'test.sequential('\''normal control'\'', () => { expect(() => { throw new Error('\'''\''); }).toThrow('\'''\''); });
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
! Warnings:
  - Vitest v5: 2 review items

src/add.js
  1:1 REVIEW [global-api-ownership] Resolve the Vitest project ownership of this file before migrating its affected global APIs. Global API edits were not applied because config selection, file scope, or globals settings are unresolved or conflicting.
    Docs: https://viteplus.dev/guide/vitest-v5#resolve-migration-findings

vite.config.ts
  10:37 REVIEW [global-api-ownership] Resolve test.includeSource patterns before migrating in-source global APIs. In-source test ownership is not statically known.
    Docs: https://viteplus.dev/guide/vitest-v5#resolve-migration-findings
```

## `vpt print-file src/add.js control.test.js`

```
export const add = (a, b) => a + b;
if (import.meta.vitest) { test.sequential('in-source', () => { expect(add(2, 3)).toBe(5); }); }
test('normal control', { concurrent: false }, () => { expect(() => { throw new Error(''); }).toThrow(/^$/); });
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

Vitest v5: 2 review items

src/add.js
  1:1 REVIEW [global-api-ownership] Resolve the Vitest project ownership of this file before migrating its affected global APIs. Global API edits were not applied because config selection, file scope, or globals settings are unresolved or conflicting.
    Docs: https://viteplus.dev/guide/vitest-v5#resolve-migration-findings

vite.config.ts
  10:37 REVIEW [global-api-ownership] Resolve test.includeSource patterns before migrating in-source global APIs. In-source test ownership is not statically known.
    Docs: https://viteplus.dev/guide/vitest-v5#resolve-migration-findings
This project is already using Vite+! Happy coding!
```
