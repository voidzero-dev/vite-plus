# package_json_not_exported

Version reporting and delegation both require the local package.json export.

## `vpt mkdir -p outer/external/inner/node_modules`


## `vpt cp -r outer-cli outer/external/inner/node_modules/vite-plus`


## `vpt write-file outer/external/inner/node_modules/vite-plus/package.json '{"name":"vite-plus","version":"9.8.7","exports":{".":"./dist/bin.js"}}'`


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
