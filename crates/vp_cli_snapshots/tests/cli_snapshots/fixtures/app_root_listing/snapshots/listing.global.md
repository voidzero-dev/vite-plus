# listing

A bare app command at a workspace root without an interactive terminal prints
the ranked package listing with -C hints and exits 1 instead of building the
root (rfcs/cwd-flag.md).

## `vp build`

**Exit code:** 1

```
[1m[2mnote:[0m[0m You are running [94m`vp build`[39m as a Vite+ built-in command. If you meant to run the build npm script, use [94m`vpr build`[39m instead.
[1m[31merror:[39m[0m `vp build` at the workspace root needs a target package.

  Packages in this workspace:
    admin  apps/admin
    web    apps/web
    ui     packages/ui

  Pass a directory:  vp -C apps/admin build
  Or run every package's build script:  vp run -r build
```

## `vp dev`

dev at the root no longer starts a server against the root

**Exit code:** 1

```
[1m[31merror:[39m[0m `vp dev` at the workspace root needs a target package.

  Packages in this workspace:
    admin  apps/admin
    web    apps/web
    ui     packages/ui

  Pass a directory:  vp -C apps/admin dev
  Or run every package's dev script:  vp run -r dev
```
