# migration_eslint_legacy_already_vite_plus

## `vp migrate --full --no-interactive`

full migration of an existing Vite+ project should warn about the legacy ESLint config and skip ESLint migration

```
VITE+ - The Unified Toolchain for the Web

▲ Legacy ESLint configuration detected (.eslintrc). Automatic migration to Oxlint requires ESLint v9+ with flat config format (eslint.config.*). Please upgrade to ESLint v9 first: https://eslint.org/docs/latest/use/migrate-to-9.0.0
◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  latest → <version>
    vite              → <version>
• 2 config updates applied
• Package manager settings configured
```

## `vpt print-file package.json`

check eslint and its lint script are kept

```
{
  "name": "migration-eslint-legacy-already-vite-plus",
  "scripts": {
    "lint": "eslint .",
    "prepare": "vp config"
  },
  "devDependencies": {
    "eslint": "^8.23.1",
    "vite-plus": "catalog:"
  }
}
```

## `vpt stat-file .eslintrc --assert file`

check legacy config is kept

```
.eslintrc: file
```
