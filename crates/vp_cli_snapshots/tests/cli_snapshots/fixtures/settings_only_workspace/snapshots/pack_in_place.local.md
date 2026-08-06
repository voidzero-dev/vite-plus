# pack_in_place

Regression guard for the vp-config e2e shape: a single package whose
pnpm-workspace.yaml only carries settings (catalogs, minimumReleaseAge) is
a workspace root whose only runnable candidate is itself. Bare vp pack must
run in place, TTY or not, never print the target listing.

## `vp pack`

```
ℹ entry: src/index.ts
ℹ Build start
ℹ dist/index.mjs  <size> kB │ gzip: <size> kB
ℹ 1 files, total: <size> kB
✔ Build complete in <duration>
```
