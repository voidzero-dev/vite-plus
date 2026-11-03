# command_pack_picomatch_hoist_false

With pnpm `hoist: false`, `vp pack` must not depend on an undeclared runtime `picomatch` lookup from the bundled tsdown. `latest` resolves to the packed checkout.

## `vp install --ignore-scripts`


## `vp pack src/index.ts`

```
VITE+ - The Unified Toolchain for the Web

ℹ entry: src/index.ts
ℹ Build start
ℹ dist/index.mjs  <size> kB │ gzip: <size> kB
ℹ 1 files, total: <size> kB
✔ Build complete in <duration>
```
