# migration_hooks_skip_on_existing_hookspath

## `git init`


## `git config core.hooksPath .custom-hooks`


## `vp migrate --no-interactive`

should skip hooks because core.hooksPath is already set

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 config updates applied
! Warnings:
  - Git hooks not configured — core.hooksPath is already set to ".custom-hooks", skipping
```

## `vpt print-file package.json`

prepare should stay 'husky' and husky must remain in devDependencies

```
{
  "name": "migration-hooks-skip-on-existing-hookspath",
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

## `vpt print-file pnpm-workspace.yaml`

check pnpm-workspace.yaml has overrides and catalog

```
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vite-plus: <version>
overrides:
  vite: 'catalog:'
peerDependencyRules:
  allowAny:
    - vite
  allowedVersions:
    vite: '*'
```

## `git config --local core.hooksPath`

should still be .custom-hooks

```
.custom-hooks
```
