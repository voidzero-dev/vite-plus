# migration_tsx_jsdoc_mixed_template

## `vpt write-file options.js '/**
 * @template T, [U=import('\''tsx/esm/api'\'').RegisterOptions]
 * @param {T} value
 * @returns {U | undefined}
 */
export function defaults(value) { return undefined; }
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

a default after a required template parameter still needs the tsx declarations

```
VITE+ - The Unified Toolchain for the Web

● tsx scripts detected. Auto-migrating to Vite+...
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 config update applied
• tsx scripts migrated to Vite+
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
