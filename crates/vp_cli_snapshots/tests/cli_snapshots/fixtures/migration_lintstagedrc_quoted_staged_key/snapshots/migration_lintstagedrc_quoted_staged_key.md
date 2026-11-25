# migration_lintstagedrc_quoted_staged_key

A quoted `'staged'` key is existing staged config, so the standalone file must be kept instead of merged as a duplicate key.

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

## `vpt stat-file .lintstagedrc.json --assert file`

lintstagedrc.json should still exist

```
.lintstagedrc.json: file
```

## `vpt print-file vite.config.ts`

vite config should have a single staged key

```
import { defineConfig } from 'vite-plus';

export default defineConfig({
  fmt: {},
  lint: {"jsPlugins":[{"name":"vite-plus","specifier":"vite-plus/oxlint-plugin"}],"rules":{"vite-plus/prefer-vite-plus-imports":"error"},"options":{"typeAware":true,"typeCheck":true}},
  'staged': {
    '*.js': 'vp check --fix',
  },
});
```
