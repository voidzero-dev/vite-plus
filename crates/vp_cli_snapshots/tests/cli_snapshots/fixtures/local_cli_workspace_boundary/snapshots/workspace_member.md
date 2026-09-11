# workspace_member

A member can use the workspace root's declared CLI dependency.

## `vpt mkdir -p outer/node_modules`


## `vpt cp -r outer-cli outer/node_modules/vite-plus`


## `node assert-boundary.mjs outer/packages/member local`

```
Version and delegation use the workspace root CLI without an install warning.
```
