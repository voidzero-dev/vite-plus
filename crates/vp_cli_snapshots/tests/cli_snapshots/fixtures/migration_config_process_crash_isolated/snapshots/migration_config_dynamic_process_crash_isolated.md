# migration_config_dynamic_process_crash_isolated

A crashing dynamic config warns and preserves imports while the existing-Vite+ upgrade continues.

## `vpt json-edit package.json devDependencies '{"vite-plus":"latest"}'`


## `vpt json-edit package.json scripts '{"build":"vite build"}'`


## `vpt replace-file-content vite.config.ts 'import { defineConfig } from '\''vite'\'';' `


## `vpt replace-file-content vite.config.ts 'export default defineConfig({});' 'export default () => ({});'`


## `vpt write-file unit.test.ts 'import { test } from '\''vitest'\'';
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  latest → <version>
    vite              → <version>
• Package manager settings configured
! Warnings:
  - Could not load lint configuration in an isolated process. Skipped Vite/Vitest import rewrites; fix the config and re-run `vp migrate`.
```

## `vpt print-file unit.test.ts`

```
import { test } from 'vitest';
```

## `vpt print-file package.json`

```
{
  "devDependencies": {
    "vite-plus": "catalog:"
  },
  "name": "migration-config-process-crash-isolated",
  "private": true,
  "scripts": {
    "build": "vp build"
  },
  "version": "0.0.0"
}
```
