# migration_turborepo_workspace

Warn once for the workspace and preserve Turborepo scripts, dependencies, and configuration.

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

## `vpt print-file turbo.json`

```
{
  "tasks": {
    "build": {
      "dependsOn": ["^build"],
      "outputs": ["dist/**"]
    }
  }
}
```

## `vpt print-file packages/app/turbo.jsonc`

```
{
  // Keep the package's Turborepo configuration during migration.
  "extends": ["//"],
  "tasks": {
    "build": {
      "outputs": ["lib/**"]
    }
  }
}
```

## `vp migrate --no-interactive --full --no-hooks --no-agent --no-editor`

An existing Vite+ project also gets the warning with --full.

```
VITE+ - The Unified Toolchain for the Web

! Warnings:
  - Turborepo detected. Task orchestration and caching were not migrated. vp run does not read Turborepo configuration. See https://viteplus.dev/guide/run and https://viteplus.dev/guide/cache.
This project is already using Vite+! Happy coding!
```
