# command_pack_no_config

## `vp pack --no-config src/index.ts`

should build without loading vite.config.ts

```
ℹ entry: src/index.ts
ℹ Build start
ℹ dist/index.mjs  <size> kB │ gzip: <size> kB
ℹ 1 files, total: <size> kB
✔ Build complete in <duration>
```

## `vpt stat-file dist/index.mjs --assert file`

should write the bundle

```
dist/index.mjs: file
```
