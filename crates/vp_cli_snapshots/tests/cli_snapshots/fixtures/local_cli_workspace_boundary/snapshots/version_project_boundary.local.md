# version_project_boundary

Both CLI entry points reject an excluded project's ancestor install, allow workspace members to share it, and find an optional local dependency.

## `vpt mkdir -p outer/node_modules outer/external/inner/node_modules`


## `vpt cp -r outer-cli outer/node_modules/vite-plus`


## `cd outer/external/inner && vp --version`

```
VITE+ - The Unified Toolchain for the Web

vp <version>

Local vite-plus:
  vite-plus  Not found
```

## `cd outer/packages/member && vp --version`

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

## `vpt cp -r outer-cli outer/external/inner/node_modules/vite-plus`


## `vpt write-file outer/external/inner/package.json '{"name":"inner","optionalDependencies":{"vite-plus":"9.8.7"}}'`


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
