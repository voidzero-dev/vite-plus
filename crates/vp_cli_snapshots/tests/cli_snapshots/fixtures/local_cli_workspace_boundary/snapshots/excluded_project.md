# excluded_project

An independent project outside the workspace patterns cannot borrow its ancestor's CLI.

## `vpt mkdir -p outer/node_modules`


## `vpt cp -r outer-cli outer/node_modules/vite-plus`


## `node assert-boundary.mjs outer/external/inner global`

```
Version reports no local CLI; delegation uses the global CLI with an install warning.
```
