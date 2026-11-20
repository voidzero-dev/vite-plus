# core_module_identity_npm_tagged_alias

Check how npm resolves a tagged core alias, then migrate to the exact alias and build with the shared core.

## `vpt write-file package.json '{"name":"core-module-identity","private":true,"type":"module","packageManager":"npm@11.11.0","devDependencies":{"vite-plus":"latest","vite":"npm:@voidzero-dev/vite-plus-core@latest"}}
'`


## `vp install --ignore-scripts`


## `node check-npm-layout.mjs tagged`

```
Checked npm resolution against the Vite peer range declared by Vitest
```

## `vp migrate --no-interactive`


## `vp install --ignore-scripts`


## `node check-npm-layout.mjs migrated`

```
npm shares the migrated core alias between the project, CLI, and Vitest
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
