# pack_selected_output

Update the output directory of an array item found by name.

## `vpt json-edit package.json devDependencies.vite-plus ^1.7.0`


## `vpt json-edit package.json devDependencies.tsdown 0.22.14`


## `vpt write-file src/cli.ts 'export const cli = true;'`


## `vpt write-file tsdown.config.ts 'export default [{ name: '\''library'\'', entry: '\''src/library.ts'\'', outDir: '\''library-out'\'', publicDir: '\''public'\'' }, { name: '\''cli'\'', entry: '\''src/cli.ts'\'', outDir: '\''cli-out'\'', publicDir: '\''cli-resources'\'' }];'`


## `vpt write-file vite.config.ts 'import configs from '\''./tsdown.config.ts'\''; const cli = configs.find(config => config.name === '\''cli'\''); if (cli) cli.outDir = '\''release-out'\''; export default { pack: configs };'`


## `vpt mkdir -p public cli-resources`


## `vpt write-file public/asset.txt 'library asset
'`


## `vpt write-file cli-resources/asset.txt 'CLI asset
'`


## `vp install --ignore-scripts`


## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`


## `vp pack`


## `vpt stat-file library-out/library.mjs release-out/cli.mjs --assert file`

```
library-out/library.mjs: file
release-out/cli.mjs: file
```

## `vpt print-file library-out/public/asset.txt release-out/cli-resources/asset.txt`

```
library asset
CLI asset
```

## `vpt stat-file cli-out --assert missing`

```
cli-out: missing
```
