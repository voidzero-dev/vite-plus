# default_package

defaultPackage in the root config acts as an implicit -C for bare app
commands, including at a root that is not a JS workspace; vp prints a note
line and runs in the configured directory (rfcs/cwd-flag.md).

## `vp pack`

```
note: vp pack: using ./packages/ui (defaultPackage)
ℹ entry: src/index.ts
ℹ Build start
ℹ dist/index.mjs  <size> kB │ gzip: <size> kB
ℹ 1 files, total: <size> kB
✔ Build complete in <duration>
```

## `vpt list-dir packages/ui/dist`

output lands in the configured package

```
index.mjs
```
