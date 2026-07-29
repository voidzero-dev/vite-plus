# migration_hook_helpers_only

## `git init`


## `vpt mkdir -p .husky`


## `vpt write-file .husky/common.sh '#'\!'/usr/bin/env sh
'`


## `vpt write-file .husky/pre-commit.bak 'npm test
'`


## `vp migrate --no-interactive`

ignore helper and backup files

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 config updates applied
• Git hooks configured
```

## `vpt stat-file .vite-hooks/pre-commit --assert file`


## `vpt print-file .vite-hooks/pre-commit`

add the default hook

```
vp staged
```

## `vpt grep-file vite.config.ts staged:`


## `vpt print-file vite.config.ts`

add staged config

```
import { defineConfig } from 'vite-plus';

export default defineConfig({
  staged: {
    "*": "vp check --fix"
  },
  fmt: {},
  lint: {"jsPlugins":[{"name":"vite-plus","specifier":"vite-plus/oxlint-plugin"}],"rules":{"vite-plus/prefer-vite-plus-imports":"error"},"options":{"typeAware":true,"typeCheck":true}},
});
```
