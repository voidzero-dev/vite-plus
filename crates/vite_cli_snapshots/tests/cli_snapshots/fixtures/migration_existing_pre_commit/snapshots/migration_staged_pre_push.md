# migration_staged_pre_push

## `git init`


## `vpt mkdir -p .husky`


## `vpt write-file .husky/pre-push '#'\!'/usr/bin/env sh
npx lint-staged
npm test
'`


## `vp migrate --no-interactive`

migrate the existing policy

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 config updates applied
• Git hooks configured
```

## `vpt stat-file .vite-hooks/pre-commit --assert missing`


## `vpt print-file .vite-hooks/pre-push`

rewrite the same hook

```
#!/usr/bin/env sh
vp staged
npm test
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
