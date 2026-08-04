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

migration should leave an existing Husky hook untouched

```
VITE+ - The Unified Toolchain for the Web

⚠ Detected Husky — leaving its hooks, configuration, and dependencies unchanged. Migrate Husky manually before enabling Vite+ hooks.
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 config update applied
```

## `vpt print-file .husky/pre-commit`

the original hook path and commands should remain

```
#!/usr/bin/env sh
npm test
secret-scan
```

## `vpt stat-file .vite-hooks --assert-not dir`

no Vite+ hook tree should be created

```
.vite-hooks: missing
```
