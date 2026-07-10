# vp_cache_monorepo_missing

## `vp run --cache ready`

first run


## `vp run --cache ready`

second run should all hit cache

```
~/packages/lib$ vp pack ◉ cache hit, replaying
ℹ entry: src/index.ts
ℹ Build start
ℹ dist/index.mjs  <size> kB │ gzip: <size> kB
ℹ 1 files, total: <size> kB
✔ Build complete in <duration>

---
vp run: cache hit, <duration> saved.
```
