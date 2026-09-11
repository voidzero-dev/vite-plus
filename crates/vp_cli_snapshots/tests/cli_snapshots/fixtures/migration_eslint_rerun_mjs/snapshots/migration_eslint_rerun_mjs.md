# migration_eslint_rerun_mjs

## `vp migrate --no-interactive`

should detect vite-plus + eslint and auto-migrate eslint

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

check eslint removed from devDependencies and scripts rewritten

```
{
  "name": "migration-eslint-rerun-mjs",
  "scripts": {
    "lint": "eslint ."
  },
  "devDependencies": {
    "eslint": "^9.0.0",
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

## `vpt stat-file eslint.config.mjs --assert-not file`

check eslint config is removed

**Exit code:** 1

```
eslint.config.mjs: file
stat-file assertion failed
```

## `vpt print-file vite.config.mjs`

check oxlint config merged into existing vite.config.mjs (not creating vite.config.ts)

```
import { defineConfig } from 'vite-plus';

export default defineConfig({});
```
