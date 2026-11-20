# pack_array_loop

A for-of loop sets a shared output directory for both packs and their resources.

## `vpt json-edit package.json devDependencies.vite-plus ^1.7.0`


## `vpt json-edit package.json devDependencies.tsdown 0.22.14`


## `vpt write-file src/cli.ts 'export const cli = true;'`


## `vpt write-file tsdown.config.ts 'export default [{ entry: '\''src/library.ts'\'', outDir: '\''library-out'\'', publicDir: '\''public'\'' }, { entry: '\''src/cli.ts'\'', outDir: '\''cli-out'\'', publicDir: '\''cli-resources'\'' }];'`


## `vpt write-file vite.config.ts 'import configs from '\''./tsdown.config.ts'\''; for (const config of configs) { config.outDir = '\''release-out'\''; } export default { pack: configs };'`


## `vpt mkdir -p public cli-resources`


## `vpt write-file public/asset.txt 'library asset
'`


## `vpt write-file cli-resources/asset.txt 'CLI asset
'`


## `vp install --ignore-scripts`


## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`


## `vp pack`


## `vpt stat-file release-out/library.mjs release-out/cli.mjs --assert file`

```
release-out/library.mjs: file
release-out/cli.mjs: file
```

## `vpt print-file release-out/public/asset.txt release-out/cli-resources/asset.txt`

```
library asset
CLI asset
```

## `vpt stat-file library-out cli-out --assert missing`

```
library-out: missing
cli-out: missing
```
