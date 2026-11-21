# pack_concatenated_configs

Pack keeps the library resource in either concat order. The explicit tsdown release uses its original config.

## `vpt json-edit package.json devDependencies.vite-plus ^1.7.0`


## `vpt json-edit package.json devDependencies.tsdown 0.22.14`


## `vpt json-edit package.json scripts.bundle 'tsdown -c tsdown.config.ts'`


## `vpt write-file src/cli.ts 'export const cli = true;'`


## `vpt write-file tsdown.config.ts 'export default [{ name: '\''library'\'', entry: '\''src/library.ts'\'', outDir: '\''library-out'\'', publicDir: '\''public'\'' }];'`


## `vpt write-file vite.config.ts 'import configs from '\''./tsdown.config.ts'\''; const extraConfigs = [{ name: '\''cli'\'', entry: '\''src/cli.ts'\'', outDir: '\''cli-out'\'' }]; export default { pack: configs.concat(extraConfigs) };'`


## `vpt mkdir -p public`


## `vpt write-file public/resource.txt 'library resource
'`


## `vp install --ignore-scripts`


## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`


## `vp pack`


## `vpt stat-file library-out/library.mjs cli-out/cli.mjs --assert file`

```
library-out/library.mjs: file
cli-out/cli.mjs: file
```

## `vpt print-file library-out/public/resource.txt`

```
library resource
```

## `vpt rm -r library-out cli-out`


## `vpt write-file vite.config.ts 'import configs from '\''./tsdown.config.ts'\''; const extraConfigs = [{ name: '\''cli'\'', entry: '\''src/cli.ts'\'', outDir: '\''cli-out'\'' }]; export default { pack: extraConfigs.concat(configs) };'`


## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`


## `vp pack`


## `vpt stat-file library-out/library.mjs cli-out/cli.mjs --assert file`

```
library-out/library.mjs: file
cli-out/cli.mjs: file
```

## `vpt print-file library-out/public/resource.txt`

```
library resource
```

## `vpt rm -r library-out cli-out`


## `pnpm run bundle`


## `vpt stat-file library-out/library.mjs --assert file`

```
library-out/library.mjs: file
```

## `vpt print-file library-out/public/resource.txt`

```
library resource
```

## `vpt stat-file cli-out --assert missing`

```
cli-out: missing
```
