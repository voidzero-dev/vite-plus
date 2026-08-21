# migration_from_tsup_monorepo_success

## `vpt chmod +x tsdown-migrate-stub.mjs`

stub tsdown-migrate so the migration stays offline

```
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

all workspace-only tsup configs should migrate automatically

```
VITE+ - The Unified Toolchain for the Web

tsup configuration detected. Auto-migrating to tsdown...
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 5 config updates applied, 2 files had imports rewritten
• tsup config migrated to tsdown (`vp pack`)
→ Manual follow-up:
  - Please manually merge packages/b/tsdown.config.ts into packages/b/vite.config.ts, see https://viteplus.dev/guide/migrate#tsdown
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

## `vpt stat-file packages/b/tsup.config.ts --assert-not file`

package b original config is removed

```
packages/b/tsup.config.ts: missing
```

## `vpt print-file packages/b/tsdown.config.ts`

package b gets a converted config

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

## `vpt print-file packages/b/package.json`

package b uses vp pack

```
{
  "name": "b",
  "scripts": {
    "build": "vp pack"
  },
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  }
}
```
