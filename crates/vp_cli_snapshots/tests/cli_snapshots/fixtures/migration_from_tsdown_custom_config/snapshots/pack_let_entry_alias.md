# pack_let_entry_alias

A let alias appends an entry to the same config object that pack loads.

## `vpt json-edit package.json devDependencies.vite-plus ^1.7.0`


## `vpt json-edit package.json devDependencies.tsdown 0.22.14`


## `vpt write-file src/cli.ts 'export const cli = true;'`


## `vpt write-file tsdown.config.ts 'export default { entry: ['\''src/library.ts'\''], outDir: '\''library-out'\'', publicDir: '\''public'\'' };'`


## `vpt write-file vite.config.ts 'import base from '\''./tsdown.config.ts'\''; let entries = base.entry; entries.push('\''src/cli.ts'\''); export default { pack: base };'`


## `vpt mkdir -p public`


## `vpt write-file public/asset.txt 'library asset
'`


## `vp install --ignore-scripts`


## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`


## `vp pack`


## `vpt stat-file library-out/library.mjs library-out/cli.mjs --assert file`

```
library-out/library.mjs: file
library-out/cli.mjs: file
```

## `vpt print-file library-out/public/asset.txt`

```
library asset
```
