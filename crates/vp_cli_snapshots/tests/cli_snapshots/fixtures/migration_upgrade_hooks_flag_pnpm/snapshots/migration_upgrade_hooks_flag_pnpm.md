# migration_upgrade_hooks_flag_pnpm

## `git init`


## `vp migrate --hooks --no-interactive`

existing Vite+ project: upgrade plus only the git hooks action

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  0.1.21 → <version>
    vite              → <version>
• 2 config updates applied
• Git hooks configured
• Package manager settings configured
```

## `git config --local core.hooksPath`

hooks configured to .vite-hooks/_

```
.vite-hooks/_
```

## `vpt print-file .vite-hooks/pre-commit`

pre-commit hook runs vp staged

```
vp staged
```

## `vpt stat-file .nvmrc --assert file`

node version file is not migrated without --full

```
.nvmrc: file
```

## `vpt stat-file .node-version --assert-not file`

only the hooks action ran

```
.node-version: missing
```
