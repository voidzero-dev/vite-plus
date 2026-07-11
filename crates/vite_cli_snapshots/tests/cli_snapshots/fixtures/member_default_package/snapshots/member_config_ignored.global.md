# member_config_ignored

defaultPackage is a root-pointer concept: a workspace member's own config
declaring it (here pointing at a missing directory) must not redirect or
fail a command already running in that member; the pack runs in place.

## `cd packages/ui && vp pack`

```
ℹ entry: src/index.ts
ℹ Build start
ℹ dist/index.mjs  <size> kB │ gzip: <size> kB
ℹ 1 files, total: <size> kB
✔ Build complete in <duration>
```
