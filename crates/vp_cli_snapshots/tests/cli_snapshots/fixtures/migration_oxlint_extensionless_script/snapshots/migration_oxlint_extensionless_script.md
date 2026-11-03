# migration_oxlint_extensionless_script

## `vp install --ignore-scripts`


## `vp run check-plugin`

The extensionless Node script works before migration.

```
VITE+ - The Unified Toolchain for the Web

$ node bin/check-plugin ⊘ cache disabled
function
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`


## `vpt print-file package.json bin/check-plugin`

Keep the dependency used by the unchanged extensionless script.

```
{
  "name": "migration-oxlint-extensionless-script",
  "private": true,
  "scripts": {
    "check-plugin": "node bin/check-plugin"
  },
  "devDependencies": {
    "@oxlint/plugins": "1.79.0",
    "vite-plus": "catalog:"
  },
  "packageManager": "pnpm@11.24.0"
}
#!/usr/bin/env node
console.log(typeof require('@oxlint/plugins').defineRule);
```

## `vpt rm -rf node_modules`


## `vp install --ignore-scripts`


## `vp run check-plugin`

The script still resolves the plugin API after a strict pnpm reinstall.

```
VITE+ - The Unified Toolchain for the Web

$ node bin/check-plugin ⊘ cache disabled
function
```
