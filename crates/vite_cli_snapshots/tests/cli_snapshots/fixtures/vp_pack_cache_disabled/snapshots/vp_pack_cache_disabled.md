# vp_pack_cache_disabled

## `vp run hello#build`

first build


## `vpt list-dir packages/hello/dist`

should have the library

```
index.cjs
```

## `vp run hello#build`

cache disabled, no cache hit

```
~/packages/hello$ vp pack ⊘ cache disabled
ℹ entry: src/index.ts
ℹ Build start
ℹ Cleaning <n> files
ℹ dist/index.cjs  <size> kB │ gzip: <size> kB
ℹ 1 files, total: <size> kB
✔ Build complete in <duration>
```

## `vp run hello#build`

should show cache disabled

```
~/packages/hello$ vp pack ⊘ cache disabled
ℹ entry: src/index.ts
ℹ Build start
ℹ Cleaning <n> files
ℹ dist/index.cjs  <size> kB │ gzip: <size> kB
ℹ 1 files, total: <size> kB
✔ Build complete in <duration>
```
