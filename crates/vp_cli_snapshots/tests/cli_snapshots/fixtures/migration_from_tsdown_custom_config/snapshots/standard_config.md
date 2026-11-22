# standard_config

## `vpt cp tsdown.custom.config.ts tsdown.config.ts`


## `vpt rm tsdown.custom.config.ts`


## `vpt json-edit package.json scripts.bundle tsdown`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`


## `vpt print-file package.json`

```
{
  "name": "migration-from-tsdown-custom-config",
  "private": true,
  "type": "module",
  "scripts": {
    "bundle": "vp pack"
  },
  "devDependencies": {
    "vite-plus": "catalog:"
  },
  "packageManager": "pnpm@12.11.2"
}
```

## `vp run bundle`


## `vpt print-file lib-out/library.mjs`

```
// src/library.ts
const answer = 42;
export { answer };
```
