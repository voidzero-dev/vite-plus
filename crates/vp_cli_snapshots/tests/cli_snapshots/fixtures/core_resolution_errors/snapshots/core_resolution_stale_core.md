# core_resolution_stale_core

## `vpt write-file node_modules/vite/package.json '{"name":"@voidzero-dev/vite-plus-core","version":"0.0.0-stale"}
'`


## `vp dev`

**Exit code:** 1

```
error: Failed to resolve vite command: GenericFailure, Expected @voidzero-dev/vite-plus-core@<version>, but found @voidzero-dev/vite-plus-core@<version> at <workspace>/node_modules/vite/package.json. Run `vp migrate` to align the Vite alias, then run `vp install`.
```

## `vp pack index.ts`

**Exit code:** 1

```
error: Failed to resolve pack command: GenericFailure, Expected @voidzero-dev/vite-plus-core@<version>, but found @voidzero-dev/vite-plus-core@<version> at <workspace>/node_modules/vite/package.json. Run `vp migrate` to align the Vite alias, then run `vp install`.
```
