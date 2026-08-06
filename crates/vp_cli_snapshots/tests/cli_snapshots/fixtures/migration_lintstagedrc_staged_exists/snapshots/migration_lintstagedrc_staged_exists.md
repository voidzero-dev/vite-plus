# migration_lintstagedrc_staged_exists

## `git init`


## `vp migrate --no-interactive`

should warn when staged already exists in vite.config.ts

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• Git hooks configured
! Warnings:
  - .lintstagedrc.json found but "staged" already exists in vite.config.ts — please merge manually
```

## `vpt print-file package.json`

check package.json

```
{
  "name": "migration-lintstagedrc-staged-exists",
  "devDependencies": {
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

## `vpt stat-file .lintstagedrc.json --assert file`

lintstagedrc.json should still exist

```
.lintstagedrc.json: file
```

## `vpt print-file vite.config.ts`

vite config should be unchanged

```
import { defineConfig } from 'vite-plus';

export default defineConfig({
  fmt: {},
  lint: {"jsPlugins":[{"name":"vite-plus","specifier":"vite-plus/oxlint-plugin"}],"rules":{"vite-plus/prefer-vite-plus-imports":"error"},"options":{"typeAware":true,"typeCheck":true}},
  staged: {
    '*.js': 'vp check --fix',
  },
});
```
