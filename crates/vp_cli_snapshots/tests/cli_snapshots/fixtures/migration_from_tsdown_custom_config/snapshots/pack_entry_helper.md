# pack_entry_helper

The local entry helper and pack use the same options after migration.

## `vpt json-edit package.json devDependencies.vite-plus ^1.7.0`


## `vpt json-edit package.json devDependencies.tsdown 0.22.14`


## `vpt write-file src/cli.ts 'export const cli = true;'`


## `vpt write-file tsdown.config.ts 'export default { entry: ['\''src/library.ts'\''], outDir: '\''library-out'\'', publicDir: '\''public'\'' };'`


## `vpt write-file vite.config.ts 'import pack from '\''./tsdown.config.ts'\''; function configure(options: typeof pack) { options.entry.push('\''src/cli.ts'\''); } configure(pack); export default { pack };'`


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
