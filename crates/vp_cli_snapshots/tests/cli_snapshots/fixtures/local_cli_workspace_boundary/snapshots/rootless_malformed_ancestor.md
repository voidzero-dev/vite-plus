# rootless_malformed_ancestor

Oxc rejects a malformed ancestor manifest even when a rootless workspace has a local installation.

## `vpt mkdir -p outer/external/inner/node_modules`


## `vpt cp -r outer-cli outer/external/inner/node_modules/vite-plus`


## `vpt rm outer/external/inner/package.json`


## `vpt write-file outer/external/inner/pnpm-workspace.yaml 'packages: []
'`


## `vpt write-file outer/package.json {`


## `cd outer/external/inner && vp --version`

```
VITE+ - The Unified Toolchain for the Web

vp <version>

Local vite-plus:
  vite-plus  Not found

Tools:
  vite             Not found
  rolldown         Not found
  vitest           Not found
  oxfmt            Not found
  oxlint           Not found
  oxlint-tsgolint  Not found
  tsdown           Not found

Environment:
  Package manager  pnpm latest
  Node.js          <version>
```

## `cd outer/external/inner && vp lint --version`

```
VITE+ - The Unified Toolchain for the Web

warn: This project does not use vite-plus. Learn how to migrate: https://viteplus.dev/guide/migrate
Version: <version>
```
