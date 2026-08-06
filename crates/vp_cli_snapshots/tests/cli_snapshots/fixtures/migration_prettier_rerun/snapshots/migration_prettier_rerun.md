# migration_prettier_rerun

## `vp migrate --no-interactive`

should detect vite-plus + prettier and auto-migrate prettier

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  latest → <version>
    vite              → <version>
• Package manager settings configured
• Skipped editor, hooks, and lint setup. Run `vp migrate --full` to apply them.
```

## `vpt print-file package.json`

check prettier removed from devDependencies and scripts rewritten

```
{
  "name": "migration-prettier-rerun",
  "scripts": {
    "format": "prettier --write ."
  },
  "devDependencies": {
    "prettier": "^3.0.0",
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "devEngines": {
    "packageManager": {
      "name": "pnpm",
      "version": "<version>",
      "onFail": "download"
    }
  }
}
```

## `vpt stat-file .prettierrc.json --assert-not file`

check prettier config is removed

**Exit code:** 1

```
.prettierrc.json: file
stat-file assertion failed
```

## `vpt print-file vite.config.ts`

check oxfmt config merged into vite.config.ts

**Exit code:** 1

```
vite.config.ts: not found
missing file
```
