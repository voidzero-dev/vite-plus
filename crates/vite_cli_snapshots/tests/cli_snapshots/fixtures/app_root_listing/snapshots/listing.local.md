# listing

A bare app command at a workspace root with several candidate packages prints
the ranked package listing with -C hints and exits 1 instead of building the
root, even in an interactive terminal (rfcs/cwd-flag.md).

## `vp build`

**Exit code:** 1

```
error: `vp build` at the workspace root needs a target package.

  Packages in this workspace:
    admin  apps/admin
    web    apps/web
    ui     packages/ui

  Pass a directory:  vp -C apps/admin build
  Or run every package's build script:  vp run -r build
```
