# migration_lintstagedrc_merge_fail

## `git init`


## `vp migrate --no-interactive`

should handle merge failure gracefully

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
! Warnings:
  - Failed to merge staged config into vite.config.ts
  - Git hooks not configured — Failed to merge staged config into vite.config.ts

Please add staged config to vite.config.ts manually, see https://viteplus.dev/guide/migrate#lint-staged
→ Manual follow-up:
  - Please add staged config to vite.config.ts manually, see https://viteplus.dev/guide/migrate#lint-staged
```

## `vpt print-file package.json`

check package.json

```
{
  "name": "migration-lintstagedrc-merge-fail",
  "devDependencies": {
    "lint-staged": "^16.2.6",
    "vite": "catalog:",
    "vite-plus": "catalog:"
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

## `vpt print-file .lintstagedrc.json`

config file should be preserved when merge fails

```
{
  "*.css": "stylelint --fix"
}
```

## `vpt print-file vite.config.ts`

vite config should be unchanged (merge failed)

```
const config = { plugins: [] };
module.exports = config;
```

## `vpt stat-file .vite-hooks/pre-commit --assert-not file`

no pre-commit hook when merge fails

**Exit code:** 1

```
.vite-hooks/pre-commit: file
stat-file assertion failed
```
