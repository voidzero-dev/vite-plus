# migrate_vitest4_community_provider

## `vpt write-file package.json '{"name":"vitest4-browser","private":true,"type":"module","packageManager":"pnpm@10.33.0","devDependencies":{"vitest":"4.1.11","@vitest/browser-webdriverio":"4.1.11","webdriverio":"^9.20.0"}}'`


## `vpt write-file vite.config.ts 'import { defineConfig } from '\''vitest/config'\'';
import { webdriverio } from '\''@vitest/browser-webdriverio'\'';
export default defineConfig({ test: { browser: { enabled: true, provider: webdriverio(), instances: [{ browser: '\''chrome'\'' }] } } });
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 file had imports rewritten
```

## `vpt print-file package.json`

```
{"name":"vitest4-browser","private":true,"type":"module","packageManager":"pnpm@10.33.0","devDependencies":{"vitest":"catalog:","@vitest/browser-webdriverio":"^5.0.0","webdriverio":"^9.20.0","vite-plus":"catalog:"}}
```

## `vpt print-file vite.config.ts`

```
import { defineConfig } from 'vite-plus';
import { webdriverio } from '@vitest/browser-webdriverio';
export default defineConfig({
  fmt: {},
  lint: {"jsPlugins":[{"name":"vite-plus","specifier":"vite-plus/oxlint-plugin"}],"rules":{"vite-plus/prefer-vite-plus-imports":"error"},"options":{"typeAware":true,"typeCheck":true}},
  test: {
      // Vitest v4 compatibility: preserve mock call history.
      // Remove after tests no longer rely on calls from setup or earlier tests.
      // https://vitest.dev/guide/migration/#clearmocks-is-enabled-by-default
      clearMocks: false,
      browser: {
        locators: {
          // Vitest v4 compatibility: keep partial, case-insensitive locator matching.
          // Remove after updating locators for full, case-sensitive matches.
          // https://vitest.dev/guide/migration/#locators-are-strict-by-default
          exact: false
        },
        enabled: true, provider: webdriverio(), instances: [{ browser: 'chrome' }] } }
});
```

## `vpt write-file node_modules/vitest/package.json '{"name":"vitest","version":"5.0.1"}'`

simulate the upgraded runner because snapshot fixtures skip install


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```
