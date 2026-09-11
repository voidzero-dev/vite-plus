# migration_eslint_monorepo_plugins_in_packages

## `vp migrate --no-interactive`

workspace packages get the SAME aggressive cleanup as the root (deps, configs, lint-staged)

```
VITE+ - The Unified Toolchain for the Web

✔ Created vite.config.ts in vite.config.ts

✔ Merged .oxlintrc.json into vite.config.ts
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 3 config updates applied
• ESLint rules migrated to Oxlint
```

## `vpt print-file package.json`

root: eslint + eslint-config-airbnb removed

```
{
  "name": "migration-eslint-monorepo-plugins-in-packages",
  "scripts": {
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

workspace: eslint, eslint-plugin-vue, @typescript-eslint/parser removed; @typescript-eslint/utils preserved (reusable AST lib)

```
{
  "name": "@test/app",
  "scripts": {
    "dev": "vp dev",
    "lint": "vp lint ."
  },
  "devDependencies": {
    "@typescript-eslint/utils": "^8.0.0",
    "vite": "catalog:",
    "vite-plus": "catalog:"
  }
}
```

## `vpt print-file packages/lint-config/package.json`

workspace: all eslint-plugin-* removed; peerDeps.eslint scrubbed (field deleted when empty)

```
{
  "name": "@test/lint-config",
  "scripts": {
    "lint": "vp lint ."
  }
}
```

## `vpt stat-file packages/app/eslint.config.mjs --assert-not file`

workspace eslint config is deleted

```
packages/app/eslint.config.mjs: missing
```

## `vpt print-file packages/app/.lintstagedrc.json`

workspace lint-staged rewritten (eslint --fix → vp lint --fix)

```
{
  "*.ts": "vp lint --fix"
}
```
