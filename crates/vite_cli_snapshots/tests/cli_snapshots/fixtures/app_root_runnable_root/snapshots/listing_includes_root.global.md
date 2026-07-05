# listing_includes_root

The non-interactive listing offers the runnable root as a `.` row.

## `vp build`

**Exit code:** 1

```
[1m[31merror:[39m[0m `vp build` at the workspace root needs a target package.

  Packages in this workspace:
    runnable-root  .
    ui             packages/ui

  Pass a directory:  vp -C . build
  Or run every package's build script:  vp run -r build
```
