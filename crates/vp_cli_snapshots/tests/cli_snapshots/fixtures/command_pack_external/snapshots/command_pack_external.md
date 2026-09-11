# command_pack_external

## `vp pack --deps.never-bundle node:path src/index.ts`

should bundle with deps.never-bundle flag

```
ℹ entry: src/index.ts
ℹ Build start
ℹ dist/index.mjs  <size> kB │ gzip: <size> kB
ℹ 1 files, total: <size> kB
✔ Build complete in <duration>
```

## `vp pack --external node:path src/index.ts`

should bundle with legacy external flag

```
ℹ entry: src/index.ts
warn: `external` is deprecated. Use `deps.neverBundle` instead.
ℹ Build start
ℹ Cleaning <n> files
ℹ dist/index.mjs  <size> kB │ gzip: <size> kB
ℹ 1 files, total: <size> kB
✔ Build complete in <duration>
```
