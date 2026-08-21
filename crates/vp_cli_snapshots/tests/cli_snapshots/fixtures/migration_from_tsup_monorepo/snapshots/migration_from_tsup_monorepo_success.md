# migration_from_tsup_monorepo_success

## `vpt chmod +x tsdown-migrate-stub.mjs`

stub tsdown-migrate so the migration stays offline

```
```

## `vpt rm -rf packages/b`

keep the success case to one workspace package


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

workspace-only tsup configs should migrate automatically

```
VITE+ - The Unified Toolchain for the Web

tsup configuration detected. Auto-migrating to tsdown...
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 3 config updates applied, 1 file had imports rewritten
• tsup config migrated to tsdown (`vp pack`)
→ Manual follow-up:
  - Please manually merge packages/a/tsdown.config.ts into packages/a/vite.config.ts, see https://viteplus.dev/guide/migrate#tsdown
```

## `vpt stat-file packages/a/tsup.config.ts --assert-not file`

package a original config is removed

```
packages/a/tsup.config.ts: missing
```

## `vpt print-file packages/a/tsdown.config.ts`

package a gets a converted config

```
import { defineConfig } from 'vite-plus/pack';

export default defineConfig({
  entry: ['src/index.ts'],
  dts: true,
  format: 'cjs',
  clean: false,
  target: false,
});
```

## `vpt print-file packages/a/package.json`

package a uses vp pack

```
{
  "name": "a",
  "scripts": {
    "build": "vp pack"
  },
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  }
}
```
