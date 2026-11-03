# v5_screenshot_directory

Do not copy an existing v5 browser screenshot directory to the separate screenshot matcher configuration.

## `vpt json-edit package.json devDependencies.vitest 5.0.1`


## `vpt write-file vite.config.ts 'export default { test: { browser: { screenshotDirectory: '\''screens'\'' } } };
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
  test: { browser: { screenshotDirectory: 'screens' } }
}
```
