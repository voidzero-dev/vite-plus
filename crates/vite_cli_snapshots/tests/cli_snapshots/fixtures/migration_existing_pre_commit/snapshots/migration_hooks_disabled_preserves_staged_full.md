# migration_hooks_disabled_preserves_staged_full

## `git init`


## `git config core.hooksPath .husky/_`


## `vpt mkdir -p .husky`


## `vpt write-file .husky/pre-commit 'npx lint-staged
'`


## `vpt json-edit package.json devDependencies.lint-staged '"^16.2.7"'`


## `vpt write-file .lintstagedrc.json '{"*.ts":"eslint --fix"}
'`


## `VITE_GIT_HOOKS=0 vp migrate --hooks --no-interactive`

skip hooks before the full config rewrite

```
VITE+ - The Unified Toolchain for the Web

⚠ Git hooks are disabled through VITE_GIT_HOOKS=0 — skipping git hooks setup.
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 config update applied
```

## `git config --local core.hooksPath`

keep the Husky dispatcher

```
.husky/_
```

## `vpt print-file .husky/pre-commit`

keep the active lint-staged hook

```
npx lint-staged
```

## `vpt print-file .lintstagedrc.json`

keep standalone staged config

```
{"*.ts":"eslint --fix"}
```

## `vpt print-file vite.config.ts`

do not add staged config

```
import { defineConfig } from 'vite-plus';

export default defineConfig({
  fmt: {},
  lint: {"jsPlugins":[{"name":"vite-plus","specifier":"vite-plus/oxlint-plugin"}],"rules":{"vite-plus/prefer-vite-plus-imports":"error"},"options":{"typeAware":true,"typeCheck":true}},
});
```

## `vpt print-file package.json`

keep hook dependencies and prepare script

```
{
  "devDependencies": {
    "husky": "^9.1.7",
    "lint-staged": "^16.2.7",
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "name": "migration-existing-pre-commit",
  "scripts": {
    "prepare": "husky"
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
