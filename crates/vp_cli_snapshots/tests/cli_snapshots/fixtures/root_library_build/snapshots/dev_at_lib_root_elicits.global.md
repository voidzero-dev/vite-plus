# dev_at_lib_root_elicits

The build-config signal is build-only: a lib/SSR root has no app to serve,
so bare vp dev at that root must still elicit (no index.html), even though
vp build runs it in place.

## `vp dev`

**Exit code:** 1

```
[1m[31merror:[39m[0m `vp dev` at the workspace root needs a target package.

  Packages in this workspace:
    ui  packages/ui

  Pass a directory:  vp -C packages/ui dev
  Or run every package's dev script:  vp run -r dev
```
