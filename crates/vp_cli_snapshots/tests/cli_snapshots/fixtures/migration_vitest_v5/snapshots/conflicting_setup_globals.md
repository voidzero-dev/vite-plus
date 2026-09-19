# conflicting_setup_globals

Preserve a setup file shared by projects with conflicting globals settings and retain its ownership review on repeated migration.

## `vpt write-file vite.config.ts 'export default { test: { projects: [{ test: { globals: true, setupFiles: '\''./setup.js'\'' } }, { test: { globals: false, setupFiles: '\''./setup.js'\'' } }] } };
'`


## `vpt write-file setup.js 'beforeEach(() => { expect(Promise.resolve(1)).resolves.toBe(1); });
'`


## `vpt write-file ordinary.test.js 'test('\''works'\'', () => { expect(1).toBe(1); expect(2).toBe(2); });
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 file had imports rewritten
! Warnings:
  - Vitest v5: 1 review item

setup.js
  1:1 REVIEW [global-api-ownership] Resolve the Vitest project ownership of this file before migrating its affected global APIs. Global API edits were not applied because config selection, file scope, or globals settings are unresolved or conflicting.
    Docs: https://viteplus.dev/guide/vitest-v5#resolve-migration-findings
```

## `vpt print-file setup.js`

```
beforeEach(() => { expect(Promise.resolve(1)).resolves.toBe(1); });
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

Vitest v5: 1 review item

setup.js
  1:1 REVIEW [global-api-ownership] Resolve the Vitest project ownership of this file before migrating its affected global APIs. Global API edits were not applied because config selection, file scope, or globals settings are unresolved or conflicting.
    Docs: https://viteplus.dev/guide/vitest-v5#resolve-migration-findings
This project is already using Vite+! Happy coding!
```
