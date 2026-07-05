# listing

A bare app command at a workspace root without an interactive terminal prints
the ranked package listing with -C hints and exits 1 instead of building the
root (rfcs/cwd-flag.md).

## `vp build`

**Exit code:** 1

```
[1m[31merror:[39m[0m `vp build` at the workspace root needs a target package.

  Packages in this workspace:
    admin  apps/admin
    web    apps/web
    ui     packages/ui

  Pass a directory:  vp -C apps/admin build
  Or run every package's build script:  vp run -r build
```
