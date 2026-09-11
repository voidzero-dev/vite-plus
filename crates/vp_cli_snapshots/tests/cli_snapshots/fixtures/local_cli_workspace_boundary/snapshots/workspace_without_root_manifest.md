# workspace_without_root_manifest

A pnpm workspace without a root package.json cannot borrow an outer project's CLI, but can use its own install.

## `vpt mkdir -p outer/node_modules outer/external/inner/apps/app`


## `vpt cp -r outer-cli outer/node_modules/vite-plus`


## `vpt rm outer/external/inner/package.json`


## `vpt write-file outer/external/inner/pnpm-workspace.yaml 'packages:
  - apps/*
'`


## `vpt write-file outer/external/inner/apps/app/package.json '{"name":"app","devDependencies":{"vite-plus":"0.3.1"}}'`


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

warn: No project-local vite-plus installation was found. Run `vp install` in `<workspace>/outer/external/inner` to install dependencies.
Version: 1.81.0
```

## `cd outer/external/inner/apps/app && vp --version`

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

## `vpt mkdir -p outer/external/inner/node_modules`


## `vpt cp -r outer-cli outer/external/inner/node_modules/vite-plus`


## `vpt write-file outer/external/inner/node_modules/vite-plus/dist/bin.js 'console.log('\''Workspace-local CLI executed.'\'')
'`


## `cd outer/external/inner && vp --version`

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
  Package manager  pnpm latest
  Node.js          <version>
```

## `cd outer/external/inner && vp lint --version`

```
VITE+ - The Unified Toolchain for the Web

Workspace-local CLI executed.
```
