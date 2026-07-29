# migration_staged_config_with_custom_hook

## `git init`


## `vpt json-edit package.json lint-staged '{"*.ts":"eslint --fix"}'`


## `vpt json-edit package.json devDependencies.lint-staged '"^16.2.7"'`


## `vpt mkdir -p .husky`


## `vpt write-file .husky/pre-push '#'\!'/usr/bin/env sh
npm test
'`


## `vp migrate --no-interactive`

migrate config without changing policy

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 config updates applied
• Git hooks configured
```

## `vpt stat-file .vite-hooks/pre-commit --assert missing`


## `vpt print-file .vite-hooks/pre-push`

keep the custom hook

```
#!/usr/bin/env sh
npm test
```

## `vpt grep-file vite.config.ts staged:`


## `vpt print-file vite.config.ts`

migrate staged config

```
import { defineConfig } from 'vite-plus';

export default defineConfig({
  fmt: {},
  lint: {"jsPlugins":[{"name":"vite-plus","specifier":"vite-plus/oxlint-plugin"}],"rules":{"vite-plus/prefer-vite-plus-imports":"error"},"options":{"typeAware":true,"typeCheck":true}},
  staged: {
    "*.ts": "eslint --fix"
  },
});
```
