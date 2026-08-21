# migration_from_tsup_success

## `vpt chmod +x tsdown-migrate-stub.mjs`

stub tsdown-migrate so the migration stays offline

```
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

tsup config should migrate automatically

```
VITE+ - The Unified Toolchain for the Web

tsup configuration detected. Auto-migrating to tsdown...
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 config updates applied, 1 file had imports rewritten
• tsup config migrated to tsdown (`vp pack`)
! Warnings:
  - tsdown-migrate: The splitting option is currently unsupported in tsdown. Code splitting is always enabled and cannot be disabled.
→ Manual follow-up:
  - Please manually merge tsdown.config.ts into vite.config.ts, see https://viteplus.dev/guide/migrate#tsdown
```

## `vpt stat-file tsup.config.ts --assert-not file`

the original tsup config is removed

```
tsup.config.ts: missing
```

## `vpt print-file tsdown.config.ts`

the converted config uses vite-plus pack

```
import { defineConfig } from 'vite-plus/pack';

export default defineConfig({
  entry: ['src/index.ts'],
  dts: true,
  format: ['esm', 'cjs'],
  splitting: false,
  target: false,
});
```

## `vpt print-file vite.config.ts`

the converted config is connected to vite.config.ts

```
import tsdownConfig from './tsdown.config.js';

import { defineConfig } from 'vite-plus';

export default defineConfig({
  pack: tsdownConfig,
  fmt: {},
  lint: {"jsPlugins":[{"name":"vite-plus","specifier":"vite-plus/oxlint-plugin"}],"rules":{"vite-plus/prefer-vite-plus-imports":"error"},"options":{"typeAware":true,"typeCheck":true}},
});
```

## `vpt print-file package.json`

tsup is removed and its script uses vp pack

```
{
  "name": "migration-from-tsup",
  "scripts": {
    "build": "vp pack"
  },
  "devDependencies": {
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
