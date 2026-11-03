# supported_engine_minimum

Keep a public engine range with a supported minimum without a Node review across repeated migration.

## `vpt json-edit package.json engines.node '>= 22.19.0'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 files had imports rewritten
```

## `vpt print-file package.json`

```
{
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "engines": {
    "node": ">= 22.19.0"
  },
  "name": "migration-vitest-v5",
  "packageManager": "pnpm@11.24.0",
  "private": true,
  "scripts": {
    "test": "vp test list --no-static-parse"
  },
  "type": "module"
}
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```
