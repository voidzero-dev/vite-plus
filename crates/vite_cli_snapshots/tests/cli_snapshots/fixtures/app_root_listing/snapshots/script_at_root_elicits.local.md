# script_at_root_elicits

A root package script "build": "vp build" run through vp run gets the same
target elicitation as a direct bare invocation: the spawned vp prints the
listing and the task fails instead of silently building the root.

## `vp run build`

**Exit code:** 1

```
$ vp build ⊘ cache disabled

[1m[31merror:[39m[0m `vp build` at the workspace root needs a target package.

  Packages in this workspace:
    admin  apps/admin
    web    apps/web
    ui     packages/ui

  Pass a directory:  vp -C apps/admin build
  Or run every package's build script:  vp run -r build
```
