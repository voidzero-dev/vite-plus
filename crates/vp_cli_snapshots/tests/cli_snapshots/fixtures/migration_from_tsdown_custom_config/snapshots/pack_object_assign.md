# pack_object_assign

Build the copied pack options with their resources and the final output directory.

## `vpt json-edit package.json devDependencies.vite-plus ^1.7.0`


## `vpt json-edit package.json devDependencies.tsdown 0.22.14`


## `vpt write-file tsdown.config.ts 'export default { entry: '\''src/library.ts'\'', outDir: '\''library-out'\'', publicDir: '\''public'\'' };'`


## `vpt write-file vite.config.ts 'import base from '\''./tsdown.config.ts'\''; export default { pack: Object.assign({}, base, { outDir: '\''release-out'\'' }) };'`


## `vpt mkdir -p public`


## `vpt write-file public/asset.txt 'library asset
'`


## `vp install --ignore-scripts`


## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`


## `vp pack`


## `vpt stat-file release-out/library.mjs --assert file`

```
release-out/library.mjs: file
```

## `vpt print-file release-out/public/asset.txt`

```
library asset
```

## `vpt stat-file library-out --assert missing`

```
library-out: missing
```
