# version_manifest_resolution

Both CLI entry points handle package exports, BOMs, and malformed ancestors through the same resolver.

## `vpt mkdir -p outer/external/inner/node_modules`


## `vpt cp -r outer-cli outer/external/inner/node_modules/vite-plus`


## `vpt write-file outer/external/inner/node_modules/vite-plus/package.json '{"name":"vite-plus","version":"9.8.7","exports":{".":"./dist/bin.js"}}'`


## `cd outer/external/inner && vp --version`

```
VITE+ - The Unified Toolchain for the Web

vp <version>

Local vite-plus:
  vite-plus  Not found
```

## `vpt write-file outer/external/inner/node_modules/vite-plus/package.json '﻿{"name":"vite-plus","version":"9.8.7"}'`


## `cd outer/external/inner && vp --version`

```
VITE+ - The Unified Toolchain for the Web

vp <version>

Local vite-plus:
  vite-plus  <version>

Tools:
  vite             Not found
  rolldown         Not found
  vitest           Not found
  oxfmt            Not found
  oxlint           Not found
  oxlint-tsgolint  Not found
  tsdown           Not found
```

## `vpt rm outer/external/inner/package.json`


## `vpt write-file outer/external/inner/pnpm-workspace.yaml 'packages: []
'`


## `cd outer/external/inner && vp --version`

```
VITE+ - The Unified Toolchain for the Web

vp <version>

Local vite-plus:
  vite-plus  <version>

Tools:
  vite             Not found
  rolldown         Not found
  vitest           Not found
  oxfmt            Not found
  oxlint           Not found
  oxlint-tsgolint  Not found
  tsdown           Not found
```

## `vpt write-file outer/package.json {`


## `cd outer/external/inner && vp --version`

```
VITE+ - The Unified Toolchain for the Web

vp <version>

Local vite-plus:
  vite-plus  Not found
```
