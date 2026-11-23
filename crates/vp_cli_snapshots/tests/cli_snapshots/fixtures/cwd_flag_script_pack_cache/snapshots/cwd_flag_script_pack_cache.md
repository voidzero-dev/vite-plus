# cwd_flag_script_pack_cache

A cached root task runs vp -C packages/app pack. Pack excludes its own dist
output from the cache inputs, and that exclusion must apply to packages/app/dist
rather than the root dist, which the package reads as an input.

## `vpt write-file dist/shared.js 'export const value = '\''shared-one'\'';
'`

dist is ignored by git, so the root input is created here

```
```

## `vp run pack:app`

first run is a cache miss

```
$ vp -C packages/app pack
ℹ entry: src/index.ts
ℹ Build start
ℹ dist/index.mjs  <size> kB │ gzip: <size> kB
ℹ 1 files, total: <size> kB
✔ Build complete in <duration>
```

## `vp run pack:app`

second run without changes is a cache hit

```
$ vp -C packages/app pack ◉ cache hit, replaying
ℹ entry: src/index.ts
ℹ Build start
ℹ dist/index.mjs  <size> kB │ gzip: <size> kB
ℹ 1 files, total: <size> kB
✔ Build complete in <duration>

---
vp run: cache hit, <duration> saved.
```

## `vpt write-file dist/shared.js 'export const value = '\''shared-two'\'';
'`

```
```

## `vp run pack:app`

changing the root dist input is a cache miss

```
$ vp -C packages/app pack ○ cache miss: 'dist/shared.js' modified, executing
ℹ entry: src/index.ts
ℹ Build start
ℹ Cleaning <n> files
ℹ dist/index.mjs  <size> kB │ gzip: <size> kB
ℹ 1 files, total: <size> kB
✔ Build complete in <duration>
```

## `vpt grep-file packages/app/dist/index.mjs shared-two`

```
packages/app/dist/index.mjs: found "shared-two"
```
