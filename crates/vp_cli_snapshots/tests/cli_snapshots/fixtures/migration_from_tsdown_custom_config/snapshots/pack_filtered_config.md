# pack_filtered_config

The pack array filter keeps the library and its resources.

## `vpt json-edit package.json devDependencies.vite-plus ^1.7.0`


## `vpt json-edit package.json devDependencies.tsdown 0.22.14`


## `vpt write-file src/cli.ts 'export const cli = true;'`


## `vpt write-file tsdown.config.ts 'export default [{ name: '\''library'\'', entry: '\''src/library.ts'\'', outDir: '\''library-out'\'', publicDir: '\''public'\'' }, { name: '\''demo'\'', entry: '\''src/cli.ts'\'', outDir: '\''cli-out'\'', publicDir: '\''cli-resources'\'' }];'`


## `vpt write-file vite.config.ts 'import configs from '\''./tsdown.config.ts'\''; export default { pack: configs.filter(config => config.name '\!'== '\''demo'\'') };'`


## `vpt mkdir -p public cli-resources`


## `vpt write-file public/asset.txt 'library asset
'`


## `vpt write-file cli-resources/asset.txt 'CLI asset
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

## `vpt stat-file cli-out --assert missing`

```
cli-out: missing
```
