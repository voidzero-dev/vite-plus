# api_equivalent_migration

Deduplicate equal static API settings despite different property order and spelling, retaining restricted access.

## `vpt write-file vite.config.ts 'export default { test: { api: { port: 51204, host: '\''localhost'\'', allowExec: false }, browser: { enabled: false, api: { allowExec: false, '\''host'\'': '\''localhost'\'', '\''port'\'': 51204 } } } };
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 file had imports rewritten
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
      api: { port: 51204, host: 'localhost', allowExec: false }, browser: {
        locators: {
          // Vitest v4 compatibility: keep partial, case-insensitive locator matching.
          // Remove after updating locators for full, case-sensitive matches.
          // https://viteplus.dev/guide/vitest-v5#remove-unneeded-compatibility-settings
          // https://vitest.dev/guide/migration/#locators-are-strict-by-default
          exact: false
        },
        enabled: false  } }
}
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```
