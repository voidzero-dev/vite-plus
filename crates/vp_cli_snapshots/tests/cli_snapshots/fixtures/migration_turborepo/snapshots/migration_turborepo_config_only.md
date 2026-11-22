# migration_turborepo_config_only

A package-level turbo.jsonc is sufficient without a root config or turbo dependency.

## `vpt json-edit package.json devDependencies {}`


## `vpt rm turbo.json`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  npm <version>
• 1 config update applied
! Warnings:
  - Turborepo detected. Task orchestration and caching were not migrated. vp run does not read Turborepo configuration. See https://viteplus.dev/guide/run and https://viteplus.dev/guide/cache.
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
