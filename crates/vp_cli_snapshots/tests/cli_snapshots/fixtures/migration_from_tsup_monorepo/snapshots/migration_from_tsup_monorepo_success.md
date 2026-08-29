# migration_from_tsup_monorepo_success

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

the real tsdown-migrate package should migrate all workspace-only tsup configs

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
  "type": "module",
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
  "type": "module",
  "scripts": {
    "build": "vp pack"
  },
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  }
}
```

## `cd packages/a && vp run build`

package a migrated vp pack script builds successfully


## `vpt list-dir packages/a/dist`

package a build artifacts are created

```
index.cjs
index.d.cts
```

## `cd packages/b && vp run build`

package b migrated vp pack script builds successfully


## `vpt list-dir packages/b/dist`

package b build artifacts are created

```
index.cjs
index.d.cts
```
