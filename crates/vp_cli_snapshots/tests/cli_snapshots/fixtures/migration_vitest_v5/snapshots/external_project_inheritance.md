# external_project_inheritance

Preserve external clearMocks and locator settings, and retain the unresolved inheritance review after migration.

## `vpt write-file base.mjs 'export default { test: { clearMocks: true, browser: { enabled: false, locators: { exact: true } } } };
'`


## `vpt write-file vite.config.ts 'export default { test: { projects: [{ extends: '\''./base.mjs'\'', test: { name: '\''unit'\'', browser: { enabled: false } } }] } };
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

Vitest v5: 1 review item

vite.config.ts
  1:39 REVIEW [project-inheritance] Review this external or dynamic project base before adding v4 compatibility defaults. Inherited project settings were left unchanged.
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 file had imports rewritten
! Warnings:
  - Vitest v5: 1 review item

vite.config.ts
  4:69 REVIEW [project-inheritance] Review this external or dynamic project base before adding v4 compatibility defaults. Inherited project settings were left unchanged.
```

## `vpt print-file vite.config.ts`

```
export default {
  fmt: {},
  lint: {"jsPlugins":[{"name":"vite-plus","specifier":"vite-plus/oxlint-plugin"}],"rules":{"vite-plus/prefer-vite-plus-imports":"error"},"options":{"typeAware":true,"typeCheck":true}},
  test: { clearMocks: false, sharedViteServer: false,  projects: [{ extends: './base.mjs', test: { name: 'unit', browser: { enabled: false } } }] }
}
```

## `vpt print-file base.mjs`

```
export default { test: { clearMocks: true, browser: { enabled: false, locators: { exact: true } } } };
```

## `node verify-review.mjs --inheritance`

```
External base settings preserved: clearMocks=true, browser.locators.exact=true
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

Vitest v5: 1 review item

vite.config.ts
  4:69 REVIEW [project-inheritance] Review this external or dynamic project base before adding v4 compatibility defaults. Inherited project settings were left unchanged.
This project is already using Vite+! Happy coding!
```
