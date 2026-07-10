# migration_pre_commit_env_setup

## `git init`


## `vpt mkdir -p .husky`


## `vpt write-file .husky/pre-commit '#'\!'/usr/bin/env sh
export NODE_OPTIONS="--max-old-space-size=4096"
npx lint-staged
npm test
'`


## `vpt chmod 755 .husky/pre-commit`


## `vpt print-file .husky/pre-commit`

check pre-commit hook before migration

```
#!/usr/bin/env sh
export NODE_OPTIONS="--max-old-space-size=4096"
npx lint-staged
npm test
```

## `vp migrate --no-interactive`

migration should replace lint-staged in-place

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 config updates applied
• Git hooks configured
```

## `vpt print-file .vite-hooks/pre-commit`

check vp staged replaced npx lint-staged in-place

```
#!/usr/bin/env sh
export NODE_OPTIONS="--max-old-space-size=4096"
vp staged
npm test
```
