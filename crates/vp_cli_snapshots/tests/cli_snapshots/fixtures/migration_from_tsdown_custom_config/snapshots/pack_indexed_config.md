# pack_indexed_config

An indexed pack config still selects one bundle and copies its resources after migration.

## `vpt json-edit package.json devDependencies.vite-plus ^1.7.0`


## `vpt json-edit package.json devDependencies.tsdown 0.22.14`


## `vpt write-file src/cli.ts 'export const cli = true;'`


## `vpt write-file tsdown.config.ts 'export default [{ entry: '\''src/library.ts'\'', outDir: '\''library-out'\'', publicDir: '\''public'\'' }, { entry: '\''src/cli.ts'\'', outDir: '\''cli-out'\'' }];'`


## `vpt write-file vite.config.ts 'import configs from '\''./tsdown.config.ts'\''; export default { pack: configs[0] };'`


## `vpt mkdir -p public`


## `vpt write-file public/asset.txt 'library asset
'`


## `vp install --ignore-scripts`


## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`


## `vp pack`


## `vpt stat-file library-out/library.mjs --assert file`

```
library-out/library.mjs: file
```

## `vpt print-file library-out/public/asset.txt`

```
library asset
```

## `vpt stat-file cli-out/cli.mjs --assert missing`

```
cli-out/cli.mjs: missing
```
