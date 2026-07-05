# pack_default_entry

Bare vp pack at the root auto-selects the only pack-runnable package: the
library whose sole signal is tsdown's default src/index.ts entry. The app's
vite.config.ts has no pack block, so it does not count as pack-runnable
(rfcs/cwd-flag.md, "The likely-runnable heuristic"); tsdown then packs via
its default entry with no pack config at all.

## `vp pack`

```
VITE+ - The Unified Toolchain for the Web

Selected package: lib (packages/lib)
Tip: run this directly with `vp -C packages/lib pack`
ℹ entry: src/index.ts
ℹ Build start
ℹ dist/index.mjs  0.10 kB │ gzip: 0.11 kB
ℹ 1 files, total: 0.10 kB
✔ Build complete in <duration>
```

## `vpt list-dir packages/lib/dist`

output lands in the auto-selected library

```
index.mjs
```
