# migration_volta

## `vp migrate --no-interactive`

migration should detect volta.node in package.json and migrate to .node-version

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 config updates applied
• Node version manager file migrated to .node-version
→ Manual follow-up:
  - Remove the "volta" field from package.json
```

## `vpt print-file .node-version`

check .node-version is created from volta.node

```
20.19.0
```

## `vpt print-file package.json`

volta field is preserved in package.json (not removed)

```
{
  "name": "migration-volta",
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "volta": {
    "node": "20.19.0",
    "npm": "10.2.5"
  },
  "devEngines": {
    "packageManager": {
      "name": "pnpm",
      "version": "<version>",
      "onFail": "download"
    }
  },
  "scripts": {
    "prepare": "vp config"
  }
}
```
