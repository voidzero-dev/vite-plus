# pack_root_still_elicits

The pack `--root` option changes path mapping. It does not select a package or an entry.

## `vp pack --root src`

**Exit code:** 1

```
[1m[31merror:[39m[0m `vp pack` at the workspace root needs a target package.

  Packages in this workspace:
    lib                   packages/lib
    app-explicit-targets  .

  Pass a directory:  vp -C packages/lib pack
  Or run every package's pack script:  vp run -r pack
```
