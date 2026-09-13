# migration_oxlint_inline_script

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`


## `vpt print-file package.json`

The unchanged inline script still needs a direct @oxlint/plugins dependency.

```
{
  "name": "migration-oxlint-inline-script",
  "private": true,
  "scripts": {
    "check-plugin": "node -e \"console.log(typeof require('@oxlint/plugins').defineRule)\""
  },
  "devDependencies": {
    "@oxlint/plugins": "1.79.0",
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "packageManager": "pnpm@11.24.0"
}
```

## `vpt rm -rf node_modules`


## `vp install --ignore-scripts`


## `vp run check-plugin`

The script resolves its plugin API after a strict pnpm reinstall.

```
VITE+ - The Unified Toolchain for the Web

$ node -e "console.log(typeof require('@oxlint/plugins').defineRule)" ⊘ cache disabled
function
```
