# migration_conflicting_project_hook

## `git init`


## `vpt mkdir -p .husky`


## `vpt mkdir -p .vite-hooks`


## `vpt write-file .husky/pre-commit 'npm test
'`


## `vpt write-file .vite-hooks/pre-commit 'vp run lint
'`


## `vp migrate --no-interactive`

skip migration instead of overwriting either hook

```
VITE+ - The Unified Toolchain for the Web

⚠ Both .husky/pre-commit and .vite-hooks/pre-commit exist — skipping git hooks setup. Resolve the duplicate hooks and re-run migration.
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 config update applied
```

## `vpt print-file .husky/pre-commit`

keep the Husky hook

```
npm test
```

## `vpt print-file .vite-hooks/pre-commit`

keep the project-owned Vite+ hook

```
vp run lint
```

## `vpt print-file package.json`

keep Husky configured

```
{
  "name": "migration-existing-pre-commit",
  "scripts": {
    "prepare": "husky"
  },
  "devDependencies": {
    "husky": "^9.1.7",
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "devEngines": {
    "packageManager": {
      "name": "pnpm",
      "version": "<version>",
      "onFail": "download"
    }
  }
}
```
