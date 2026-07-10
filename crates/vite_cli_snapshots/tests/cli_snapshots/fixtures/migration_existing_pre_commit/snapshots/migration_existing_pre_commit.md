# migration_existing_pre_commit

## `git init`


## `vpt mkdir -p .husky`


## `vpt write-file .husky/pre-commit '#'\!'/usr/bin/env sh
npm test
secret-scan
'`


## `vpt chmod 755 .husky/pre-commit`


## `vpt print-file .husky/pre-commit`

check existing pre-commit hook before migration

```
#!/usr/bin/env sh
npm test
secret-scan
```

## `vp migrate --no-interactive`

migration should preserve existing pre-commit contents

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 config updates applied
• Git hooks configured
```

## `vpt print-file .vite-hooks/pre-commit`

check pre-commit hook preserves existing commands

```
#!/usr/bin/env sh
npm test
secret-scan
vp staged
```
