# migration_eslint_monorepo

## `vp migrate --no-interactive`

migration should detect eslint in monorepo and migrate all packages

```
VITE+ - The Unified Toolchain for the Web

✔ Created vite.config.ts in vite.config.ts

✔ Merged .oxlintrc.json into vite.config.ts
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 config updates applied
• ESLint rules migrated to Oxlint
```

## `vpt print-file package.json`

check root eslint removed and scripts rewritten

```
{
  "name": "migration-eslint-monorepo",
  "scripts": {
    "dev": "vp dev",
    "build": "vp build",
    "lint": "vp lint .",
    "prepare": "vp config"
  },
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "packageManager": "pnpm@10.18.0"
}
```

## `vpt print-file packages/app/package.json`

check app eslint removed and scripts rewritten

```
{
  "name": "app",
  "scripts": {
    "dev": "vp dev",
    "build": "vp build",
    "lint": "vp lint .",
    "lint:fix": "vp lint --fix ."
  },
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  }
}
```

## `vpt print-file packages/utils/package.json`

check utils eslint removed and scripts rewritten

```
{
  "name": "@test/utils",
  "scripts": {
    "lint": "vp lint ."
  }
}
```

## `vpt stat-file eslint.config.mjs --assert-not file`

check root eslint config is removed

```
eslint.config.mjs: missing
```
