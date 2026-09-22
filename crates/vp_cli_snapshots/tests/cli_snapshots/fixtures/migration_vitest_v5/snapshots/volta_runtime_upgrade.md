# volta_runtime_upgrade

Upgrade the Volta pin before converting it to .node-version, including the cached setup decision.

## `vpt json-edit package.json volta.node 20.19.0`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 files had imports rewritten
• Node version manager file migrated to .node-version
→ Manual follow-up:
  - Remove the "volta" field from package.json
```

## `vpt print-file .node-version package.json`

```
22.18.0
{
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "name": "migration-vitest-v5",
  "packageManager": "pnpm@11.24.0",
  "private": true,
  "scripts": {
    "test": "vp test list --no-static-parse"
  },
  "type": "module",
  "volta": {
    "node": "22.18.0"
  }
}
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```
