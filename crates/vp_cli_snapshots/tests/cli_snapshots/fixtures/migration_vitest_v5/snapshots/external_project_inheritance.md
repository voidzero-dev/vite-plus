# external_project_inheritance

Preserve external clearMocks and locator settings, and include the unresolved inheritance review in the first migration's final report.

## `vpt write-file base.mjs 'export default { test: { clearMocks: true, browser: { enabled: false, locators: { exact: true } } } };
'`


## `vpt write-file vite.config.ts 'export default { test: { projects: [{ extends: '\''./base.mjs'\'', test: { name: '\''unit'\'', browser: { enabled: false } } }] } };
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 file had imports rewritten
! Warnings:
  - Vitest v5: 1 review item

vite.config.ts
  15:20 REVIEW [project-inheritance] Review this external or dynamic project base before adding v4 compatibility defaults. Inherited project settings were left unchanged.
    Docs: https://vitest.dev/guide/migration/#inline-projects-inherit-the-root-config-by-default
```

## `vpt print-file vite.config.ts`

```
export default {
  fmt: {},
  lint: {"jsPlugins":[{"name":"vite-plus","specifier":"vite-plus/oxlint-plugin"}],"rules":{"vite-plus/prefer-vite-plus-imports":"error"},"options":{"typeAware":true,"typeCheck":true}},
  test: {
      // Vitest v4 compatibility: preserve mock call history.
      // Remove after tests no longer rely on calls from setup or earlier tests.
      // https://viteplus.dev/guide/vitest-v5#remove-unneeded-compatibility-settings
      // https://vitest.dev/guide/migration/#clearmocks-is-enabled-by-default
      clearMocks: false,
      // Vitest v4 compatibility: keep separate Vite servers for inline projects.
      // Remove when plugins and config hooks can run once for shared projects.
      // https://viteplus.dev/guide/vitest-v5#remove-unneeded-compatibility-settings
      // https://vitest.dev/guide/migration/#inline-projects-share-the-vite-server-by-default
      sharedViteServer: false,
      projects: [{ extends: './base.mjs', test: { name: 'unit', browser: { enabled: false } } }] }
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

This project is already using Vite+! Happy coding!
```
