# default_package_per_command

The object form maps commands individually: `vp build` targets ./frontend
while `vp pack` targets ./lib, so one repo can dev an app and pack a
library (rfcs/cwd-flag.md).

## `cd per_command && vp build`

```
note: vp build: using ./frontend (defaultPackage in vite.config.ts)
✓ 2 modules transformed.
computing gzip size...
dist/index.html  <size> kB │ gzip: <size> kB

✓ built in <duration>
```

## `cd per_command && vp pack`

```
note: vp pack: using ./lib (defaultPackage in vite.config.ts)
ℹ entry: src/index.ts
ℹ Build start
ℹ dist/index.mjs  <size> kB │ gzip: <size> kB
ℹ 1 files, total: <size> kB
✔ Build complete in <duration>
```
