# pack_array_callback

A callback sets both output directories on the options used by pack.

## `vpt json-edit package.json devDependencies.vite-plus ^1.7.0`


## `vpt json-edit package.json devDependencies.tsdown 0.22.14`


## `vpt write-file src/cli.ts 'export const cli = true;'`


## `vpt write-file tsdown.config.ts 'export default [{ entry: '\''src/library.ts'\'', outDir: '\''old-library-out'\'', publicDir: '\''public'\'' }, { entry: '\''src/cli.ts'\'', outDir: '\''old-cli-out'\'', publicDir: '\''cli-resources'\'' }];'`


## `vpt write-file vite.config.ts 'import configs from '\''./tsdown.config.ts'\''; configs.forEach((options, index) => { options.outDir = index === 0 ? '\''library-out'\'' : '\''cli-out'\''; }); export default { pack: configs };'`


## `vpt mkdir -p public cli-resources`


## `vpt write-file public/asset.txt 'library asset
'`


## `vpt write-file cli-resources/asset.txt 'CLI asset
'`


## `vp install --ignore-scripts`


## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`


## `vp pack`


## `vpt stat-file library-out/library.mjs cli-out/cli.mjs --assert file`

```
library-out/library.mjs: file
cli-out/cli.mjs: file
```

## `vpt print-file library-out/public/asset.txt cli-out/cli-resources/asset.txt`

```
library asset
CLI asset
```

## `vpt stat-file old-library-out old-cli-out --assert missing`

```
old-library-out: missing
old-cli-out: missing
```
