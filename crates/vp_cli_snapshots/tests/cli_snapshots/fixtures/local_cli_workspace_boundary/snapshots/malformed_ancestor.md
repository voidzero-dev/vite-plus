# malformed_ancestor

A malformed ancestor manifest cannot remove the independent project's boundary.

## `vpt mkdir -p outer/node_modules`


## `vpt cp -r outer-cli outer/node_modules/vite-plus`


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
  Package manager  Not found
  Node.js          <version>
```

## `cd outer/external/inner && vp lint --version`

the global CLI reports the malformed manifest instead of running the ancestor CLI

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

warn: No project-local vite-plus installation was found. Run `vp install` in `<workspace>/outer/external/inner` to install dependencies.
error: Failed to parse JSON file at <workspace>/outer/package.json
* EOF while parsing an object at line 1 column 1
```
