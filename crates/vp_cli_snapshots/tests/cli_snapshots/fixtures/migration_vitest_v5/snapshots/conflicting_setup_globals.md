# conflicting_setup_globals

Preserve a setup file shared by projects with conflicting globals settings and retain its ownership review on repeated migration.

## `vpt write-file vite.config.ts 'export default { test: { projects: [{ test: { globals: true, setupFiles: '\''./setup.js'\'' } }, { test: { globals: false, setupFiles: '\''./setup.js'\'' } }] } };
'`


## `vpt write-file setup.js 'beforeEach(() => { expect(Promise.resolve(1)).resolves.toBe(1); });
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

Vitest v5: 2 review items

setup.js
  1:1 REVIEW [global-api-ownership] Determine which Vitest project owns this global API before migrating it. Config selection, file scope, or globals settings are unresolved or conflicting; the global call was left unchanged.
  1:20 REVIEW [global-api-ownership] Determine which Vitest project owns this global API before migrating it. Config selection, file scope, or globals settings are unresolved or conflicting; the global call was left unchanged.
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 file had imports rewritten
! Warnings:
  - Vitest v5: 2 review items

setup.js
  1:1 REVIEW [global-api-ownership] Determine which Vitest project owns this global API before migrating it. Config selection, file scope, or globals settings are unresolved or conflicting; the global call was left unchanged.
  1:20 REVIEW [global-api-ownership] Determine which Vitest project owns this global API before migrating it. Config selection, file scope, or globals settings are unresolved or conflicting; the global call was left unchanged.
```

## `vpt print-file setup.js`

```
beforeEach(() => { expect(Promise.resolve(1)).resolves.toBe(1); });
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

Vitest v5: 2 review items

setup.js
  1:1 REVIEW [global-api-ownership] Determine which Vitest project owns this global API before migrating it. Config selection, file scope, or globals settings are unresolved or conflicting; the global call was left unchanged.
  1:20 REVIEW [global-api-ownership] Determine which Vitest project owns this global API before migrating it. Config selection, file scope, or globals settings are unresolved or conflicting; the global call was left unchanged.
This project is already using Vite+! Happy coding!
```
