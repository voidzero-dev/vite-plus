# default_package_per_command_fallthrough

A command absent from the defaultPackage object falls through to the
normal resolution: the map only declares `pack`, so bare `vp build` at
this workspace root runs in place at the (runnable) root with no note,
while `vp pack` still routes to the declared ./packages/ui.

## `cd per_command_fallthrough && vp build`

```
✓ 2 modules transformed.
computing gzip size...
dist/index.html  <size> kB │ gzip: <size> kB

✓ built in <duration>
```

## `cd per_command_fallthrough && vp pack`

```
note: vp pack: using ./packages/ui (defaultPackage in vite.config.ts)
ℹ entry: src/index.ts
ℹ Build start
ℹ dist/index.mjs  <size> kB │ gzip: <size> kB
ℹ 1 files, total: <size> kB
✔ Build complete in <duration>
```
