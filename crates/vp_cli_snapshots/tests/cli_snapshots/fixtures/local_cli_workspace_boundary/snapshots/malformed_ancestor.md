# malformed_ancestor

A malformed ancestor manifest cannot remove the independent project's boundary.

## `vpt mkdir -p outer/node_modules`


## `vpt cp -r outer-cli outer/node_modules/vite-plus`


## `vpt write-file outer/package.json {`


## `node assert-boundary.mjs outer/external/inner global`

```
Version reports no local CLI; delegation uses the global CLI with an install warning.
```
