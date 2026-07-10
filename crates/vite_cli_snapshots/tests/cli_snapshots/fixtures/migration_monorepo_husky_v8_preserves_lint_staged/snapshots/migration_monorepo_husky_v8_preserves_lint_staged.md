# migration_monorepo_husky_v8_preserves_lint_staged

## `vp migrate --no-interactive`

should warn about husky v8, preserve all lint-staged config

```
VITE+ - The Unified Toolchain for the Web

⚠ Detected husky <9.0.0 — please upgrade to husky v9+ first, then re-run migration.
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 config update applied
```

## `vpt print-file package.json`

root lint-staged config should still be in package.json

```
{
  "name": "migration-monorepo-husky-v8-preserves-lint-staged",
  "scripts": {
    "prepare": "husky install"
  },
  "devDependencies": {
    "husky": "^8.0.0",
    "lint-staged": "^15.0.0",
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "lint-staged": {
    "*.{js,ts}": "eslint --fix"
  },
  "packageManager": "pnpm@10.18.0"
}
```

## `vpt print-file packages/app/package.json`

app lint-staged config should still be in package.json

```
{
  "name": "app",
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "lint-staged": {
    "*.css": "stylelint --fix"
  }
}
```
