# migration_turborepo_dependency_only

A turbo dependency is sufficient without any Turborepo configuration files.

## `vpt rm turbo.json`


## `vpt rm packages/app/turbo.jsonc`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  npm <version>
• 1 config update applied
! Warnings:
  - Turborepo detected. Task orchestration and caching were not migrated. vp run does not read Turborepo configuration. See https://viteplus.dev/guide/run and https://viteplus.dev/guide/cache.
```

## `vpt print-file package.json`

```
{
  "name": "migration-turborepo",
  "private": true,
  "workspaces": [
    "packages/*"
  ],
  "scripts": {
    "build": "turbo run build"
  },
  "devDependencies": {
    "turbo": "^2.0.0",
    "vite-plus": "<version>"
  },
  "packageManager": "npm@11.16.0",
  "overrides": {
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>"
  }
}
```
