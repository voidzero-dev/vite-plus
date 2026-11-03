# core_resolution_upstream_vite

## `vpt write-file node_modules/vite/package.json '{"name":"vite","version":"8.2.2"}
'`


## `vp dev`

**Exit code:** 1

```
error: Failed to resolve vite command: GenericFailure, Expected @voidzero-dev/vite-plus-core@<version>, but found vite@8.2.2 at <workspace>/node_modules/vite/package.json. Run `vp migrate` to align the Vite alias, then run `vp install`.
```

## `vp pack index.ts`

**Exit code:** 1

```
error: Failed to resolve pack command: GenericFailure, Expected @voidzero-dev/vite-plus-core@<version>, but found vite@8.2.2 at <workspace>/node_modules/vite/package.json. Run `vp migrate` to align the Vite alias, then run `vp install`.
```
