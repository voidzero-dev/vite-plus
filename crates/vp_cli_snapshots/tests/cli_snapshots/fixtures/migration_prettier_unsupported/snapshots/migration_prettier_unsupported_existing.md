# migration_prettier_unsupported_existing

## `vpt json-edit package.json devDependencies.vite-plus latest`


## `vp migrate --full --no-interactive --no-agent --no-hooks`

full migration of an existing Vite+ project should skip unsupported Prettier configs

```
VITE+ - The Unified Toolchain for the Web

● Prettier configuration detected. Auto-migrating to Oxfmt...
▲ ⚠ Failed to load Prettier config at: <workspace>/.prettierrc.yml
  YAML, JSON5 and TOML formats are not supported. Convert it to JSON or JS and re-run.
● Keeping Prettier. Convert its config to JSON or JS and run `vp migrate --full` to migrate it later.
◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  latest → <version>
    vite              → <version>
• Package manager settings configured
```

## `vpt print-file package.json`

```
{
  "devDependencies": {
    "prettier": "^3.0.0",
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "name": "migration-prettier-unsupported",
  "scripts": {
    "dev": "vp dev",
    "format": "prettier --write ."
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

```
export const value={answer:42}
```

## `vpt stat-file .oxfmtrc.json --assert missing`

```
.oxfmtrc.json: missing
```
