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

migration should preserve the existing Husky hook

```
VITE+ - The Unified Toolchain for the Web

⚠ Detected Husky — leaving its hooks, configuration, and dependencies unchanged. Migrate Husky manually before enabling Vite+ hooks.
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 config update applied
```

## `vpt print-file .husky/pre-commit`

check the Husky hook is unchanged

```
#!/usr/bin/env sh
export NODE_OPTIONS="--max-old-space-size=4096"
npx lint-staged
npm test
```
