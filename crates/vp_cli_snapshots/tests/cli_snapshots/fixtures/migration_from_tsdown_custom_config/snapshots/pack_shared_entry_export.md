# pack_shared_entry_export

An entry appended through a named import is built with the original entries and resources.

## `vpt json-edit package.json devDependencies.vite-plus ^1.7.0`


## `vpt json-edit package.json devDependencies.tsdown 0.22.14`


## `vpt write-file src/cli.ts 'export const cli = true;'`


## `vpt write-file tsdown.config.ts 'export const entries = ['\''src/library.ts'\'']; export default { entry: entries, outDir: '\''library-out'\'', publicDir: '\''public'\'' };'`


## `vpt write-file vite.config.ts 'import base, { entries } from '\''./tsdown.config.ts'\''; entries.push('\''src/cli.ts'\''); export default { pack: base };'`


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
