# migration_tsx_jsdoc_name_first_param

## `vpt write-file options.js '/** @param options {import('\''tsx/esm/api'\'').RegisterOptions} */
export function withDefaults(options) { return options; }
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

a type after the parameter name still needs the tsx declarations

```
VITE+ - The Unified Toolchain for the Web

● tsx scripts detected. Auto-migrating to vpx...
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 config update applied
• tsx scripts migrated to `vpx`
! Warnings:
  - Kept the tsx devDependency in package.json: options.js still uses tsx.
```

## `vpt print-file package.json`

start runs vpx, and the tsx devDependency stays

```
{
  "name": "migration-tsx-jsdoc",
  "type": "module",
  "scripts": {
    "start": "vpx index.ts"
  },
  "devDependencies": {
    "tsx": "file:.fixture/tsx",
    "vite-plus": "catalog:"
  },
  "packageManager": "pnpm@10.33.2"
}
```
