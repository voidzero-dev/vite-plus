# pack_variable_index

An environment-selected pack config copies the resources for the selected build.

## `vpt json-edit package.json devDependencies.vite-plus ^1.7.0`


## `vpt json-edit package.json devDependencies.tsdown 0.22.14`


## `vpt write-file src/cli.ts 'export const cli = true;'`


## `vpt write-file tsdown.config.ts 'export default [{ entry: '\''src/library.ts'\'', outDir: '\''library-out'\'', publicDir: '\''public'\'' }, { entry: '\''src/cli.ts'\'', outDir: '\''cli-out'\'', publicDir: '\''cli-resources'\'' }];'`


## `vpt write-file vite.config.ts 'import configs from '\''./tsdown.config.ts'\''; const profile = process.env.BUILD_CLI === '\''1'\'' ? 1 : 0; export default { pack: configs[profile] };'`


## `vpt mkdir -p public cli-resources`


## `vpt write-file public/asset.txt 'library asset
'`


## `vpt write-file cli-resources/asset.txt 'CLI asset
'`


## `vp install --ignore-scripts`


## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`


## `BUILD_CLI=0 vp pack`


## `vpt stat-file library-out/library.mjs --assert file`

```
library-out/library.mjs: file
```

## `vpt stat-file cli-out/cli.mjs --assert missing`

```
cli-out/cli.mjs: missing
```

## `vpt print-file library-out/public/asset.txt`

```
library asset
```

## `BUILD_CLI=1 vp pack`


## `vpt stat-file cli-out/cli.mjs --assert file`

```
cli-out/cli.mjs: file
```

## `vpt print-file cli-out/cli-resources/asset.txt`

```
CLI asset
```
