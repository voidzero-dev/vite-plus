# migration_nested_hook_helper

## `git init`


## `vpt mkdir -p .husky/scripts`


## `vpt write-file .husky/pre-commit '#'\!'/usr/bin/env sh
. "$(dirname "$0")/scripts/check.sh"
'`


## `vpt write-file .husky/scripts/check.sh 'npx lint-staged
'`


## `vp migrate --no-interactive`

preserve and migrate a nested hook helper

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 config updates applied
• Git hooks configured
```

## `vpt print-file .vite-hooks/pre-commit`

keep the project hook

```
#!/usr/bin/env sh
. "$(dirname "$0")/scripts/check.sh"
```

## `vpt print-file .vite-hooks/scripts/check.sh`

copy and rewrite the nested helper

```
vp staged
```

## `vpt grep-file vite.config.ts staged:`


## `vpt print-file vite.config.ts`

add staged config discovered through the helper

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
