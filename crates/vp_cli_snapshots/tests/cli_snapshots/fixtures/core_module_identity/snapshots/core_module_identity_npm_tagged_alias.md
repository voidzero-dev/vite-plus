# core_module_identity_npm_tagged_alias

The core version satisfies Vitest's vite peer range, so npm keeps a tagged core alias instead of replacing it with upstream Vite.

## `vpt write-file package.json '{"name":"core-module-identity","private":true,"type":"module","packageManager":"npm@11.11.0","devDependencies":{"vite-plus":"latest","vite":"npm:@voidzero-dev/vite-plus-core@latest"}}
'`


## `vp install --ignore-scripts`


## `node check-tagged-alias.mjs`

```
npm keeps the tagged core alias and shares it between the project, CLI, and Vitest
```

## `vp pack entry.js`

```
VITE+ - The Unified Toolchain for the Web

ℹ entry: entry.js
ℹ Build start
ℹ dist/entry.mjs  <size> kB │ gzip: <size> kB
ℹ 1 files, total: <size> kB
✔ Build complete in <duration>
```
