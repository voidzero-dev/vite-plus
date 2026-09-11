# bom_manifest

A UTF-8 BOM does not hide the project's vite-plus dependency.

## `vpt mkdir -p outer/node_modules`


## `vpt cp -r outer-cli outer/node_modules/vite-plus`


## `vpt write-file outer/external/inner/package.json '﻿{"name":"inner","devDependencies":{"vite-plus":"0.3.1"}}'`


## `vpt write-file outer/external/inner/pnpm-workspace.yaml 'packages: []
'`


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
  Package manager  Not found
  Node.js          <version>
```

## `cd outer/external/inner && vp lint --version`

```
VITE+ - The Unified Toolchain for the Web

warn: No project-local vite-plus installation was found. Run `vp install` in `<workspace>/outer/external/inner` to install dependencies.
Version: 1.81.0
```
