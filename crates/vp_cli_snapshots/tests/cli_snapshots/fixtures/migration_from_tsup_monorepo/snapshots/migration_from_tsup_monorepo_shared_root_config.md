# migration_from_tsup_monorepo_shared_root_config

## `vpt cp packages/a/tsup.config.ts tsup.config.ts`


## `vpt rm packages/a/tsup.config.ts packages/b/tsup.config.ts`


## `vpt json-edit package.json devDependencies.tsup ^8.5.0`


## `vpt json-edit package.json devDependencies.typescript ^5.9.2`


## `vpt json-edit packages/a/package.json scripts.build 'tsup --config ../../tsup.config.ts'`


## `vpt json-edit packages/b/package.json scripts.build 'tsup --config ../../tsup.config.ts'`


## `vp install`

install the original shared-config tsup workspace


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

a shared root tsup config should be preserved with a warning

```
VITE+ - The Unified Toolchain for the Web

tsup configuration detected. Auto-migrating to tsdown...

✔ Created vite.config.ts in vite.config.ts

✔ Added import for tsdown.config.ts in vite.config.ts

Please manually merge tsdown.config.ts into vite.config.ts, see https://viteplus.dev/guide/migrate#tsdown
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 file had imports rewritten
• tsup config migrated to tsdown (`vp pack`)
! Warnings:
  - tsup.config.ts is shared by packages/a, packages/b. It was preserved and must be migrated manually.
→ Manual follow-up:
  - Please manually merge tsdown.config.ts into vite.config.ts, see https://viteplus.dev/guide/migrate#tsdown
```

## `vpt stat-file tsup.config.ts --assert file`

the shared tsup config is preserved

```
tsup.config.ts: file
```

## `vpt stat-file tsdown.config.ts --assert file`

the root also gets a migrated tsdown config

```
tsdown.config.ts: file
```

## `vpt print-file package.json`

the root keeps tsup for the shared config

```
{
  "devDependencies": {
    "typescript": "^5.9.2",
    "vite": "catalog:",
    "tsup": "^8.5.0",
    "vite-plus": "catalog:"
  },
  "name": "migration-from-tsup-monorepo",
  "packageManager": "pnpm@10.18.0",
  "private": true
}
```

## `vpt print-file packages/a/package.json`

package a keeps its shared-config tsup script

```
{
  "devDependencies": {
    "tsup": "^8.5.0",
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "name": "a",
  "scripts": {
    "build": "tsup --config ../../tsup.config.ts"
  },
  "type": "module"
}
```

## `vpt print-file packages/b/package.json`

package b keeps its shared-config tsup script

```
{
  "devDependencies": {
    "tsup": "^8.5.0",
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "name": "b",
  "scripts": {
    "build": "tsup --config ../../tsup.config.ts"
  },
  "type": "module"
}
```

## `cd packages/a && vp run build`

package a shared-config build remains usable

```
~/packages/a$ tsup --config ../../tsup.config.ts ⊘ cache disabled
CLI Building entry: src/index.ts
CLI tsup <version>
CLI Using tsup config: <workspace>/tsup.config.ts
CLI Target: node16
CJS Build start
CJS dist/index.cjs <size> B
CJS ⚡️ Build success in <duration>
DTS Build start
DTS ⚡️ Build success in <duration>
DTS dist/index.d.cts <size> B
```

## `vpt list-dir packages/a/dist`

package a build artifacts are created

```
index.cjs
index.d.cts
```

## `cd packages/b && vp run build`

package b shared-config build remains usable

```
~/packages/b$ tsup --config ../../tsup.config.ts ⊘ cache disabled
CLI Building entry: src/index.ts
CLI tsup <version>
CLI Using tsup config: <workspace>/tsup.config.ts
CLI Target: node16
CJS Build start
CJS dist/index.cjs <size> B
CJS ⚡️ Build success in <duration>
DTS Build start
DTS ⚡️ Build success in <duration>
DTS dist/index.d.cts <size> B
```

## `vpt list-dir packages/b/dist`

package b build artifacts are created

```
index.cjs
index.d.cts
```
