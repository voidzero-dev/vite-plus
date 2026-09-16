# workspace_member

A member can use the workspace root's declared CLI dependency.

## `vpt mkdir -p outer/node_modules`


## `vpt cp -r outer-cli outer/node_modules/vite-plus`


## `cd outer/packages/member && vp --version`

```
VITE+ - The Unified Toolchain for the Web

vp <version>

Local vite-plus:
  vite-plus  <version>

Tools:
  vite             <version>
  rolldown         <version>
  vitest           Not found
  oxfmt            Not found
  oxlint           Not found
  oxlint-tsgolint  Not found
  tsdown           <version>

Environment:
  Package manager  Not found
  Node.js          <version>
```

## `cd outer/packages/member && vp lint --version`

```
VITE+ - The Unified Toolchain for the Web

Ancestor workspace CLI executed.
```
