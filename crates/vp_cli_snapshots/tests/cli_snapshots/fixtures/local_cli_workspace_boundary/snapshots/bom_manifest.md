# bom_manifest

A UTF-8 BOM does not hide the project's vite-plus dependency.

## `vpt mkdir -p outer/node_modules`


## `vpt cp -r outer-cli outer/node_modules/vite-plus`


## `vpt write-file outer/external/inner/package.json '﻿{"name":"inner","devDependencies":{"vite-plus":"0.3.1"}}'`


## `vpt write-file outer/external/inner/pnpm-workspace.yaml 'packages: []
'`


## `node assert-boundary.mjs outer/external/inner global`

```
Version reports no local CLI; delegation uses the global CLI with an install warning.
```
