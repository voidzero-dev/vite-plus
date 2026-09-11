# default_package_per_command

The object form maps commands individually at a workspace root: `vp build`
targets ./apps/web while `vp pack` targets ./packages/ui, so one monorepo
can dev an app and pack a library (rfcs/cwd-flag.md).

## `cd per_command && vp build`

```
note: vp build: using ./apps/web (defaultPackage in vite.config.ts)
✓ 2 modules transformed.
computing gzip size...
dist/index.html  <size> kB │ gzip: <size> kB

✓ built in <duration>
```

## `cd per_command && vp pack`

```
note: vp pack: using ./packages/ui (defaultPackage in vite.config.ts)
ℹ entry: src/index.ts
ℹ Build start
ℹ dist/index.mjs  <size> kB │ gzip: <size> kB
ℹ 1 files, total: <size> kB
✔ Build complete in <duration>
```
