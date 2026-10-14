# migration_prettier_unsupported

## `vp migrate --no-interactive --no-agent --no-hooks`

unsupported Prettier config should not stop migration

```
VITE+ - The Unified Toolchain for the Web

● Prettier configuration detected. Auto-migrating to Oxfmt...
▲ ⚠ Failed to load Prettier config at: <workspace>/.prettierrc.yml
  YAML, JSON5 and TOML formats are not supported. Convert it to JSON or JS and re-run.
● Keeping Prettier. Convert its config to JSON or JS and run `vp migrate --full` to migrate it later.
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
✓ Dependencies installed in <duration>
• 1 config update applied
```

## `vpt print-file package.json`

keep Prettier and its format script while migrating Vite

```
{
  "name": "migration-prettier-unsupported",
  "scripts": {
    "dev": "vp dev",
    "format": "prettier --write ."
  },
  "devDependencies": {
    "prettier": "^3.0.0",
    "vite": "catalog:",
    "vite-plus": "catalog:"
  }
}
```

## `vpt print-file .prettierrc.yml`

```
semi: false
singleQuote: true
printWidth: 80
trailingComma: none
```

## `vpt print-file index.js`

do not format with Oxfmt when Prettier migration was skipped

```
export const value={answer:42}
```

## `vpt stat-file .oxfmtrc.json --assert missing`

```
.oxfmtrc.json: missing
```
