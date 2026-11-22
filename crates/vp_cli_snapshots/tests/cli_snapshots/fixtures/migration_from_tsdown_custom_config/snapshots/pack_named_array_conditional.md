# pack_named_array_conditional

A named array keeps each selected pack output directory and the library resource.

## `vpt json-edit package.json devDependencies.vite-plus ^1.7.0`


## `vpt json-edit package.json devDependencies.tsdown 0.22.14`


## `vpt json-edit package.json scripts.bundle 'tsdown -c tsdown.config.ts'`


## `vpt write-file tsdown.config.ts 'export default { name: '\''library'\'', entry: '\''src/library.ts'\'', outDir: '\''library-out'\'', publicDir: '\''public'\'' };'`


## `vpt write-file src/cli.ts 'export const cli = true;'`


## `vpt write-file vite.config.ts 'import library from '\''./tsdown.config.ts'\''; const cli = { name: '\''cli'\'', entry: '\''src/cli.ts'\'', outDir: '\''cli-out'\'' }; const formats = [library, cli]; for (const options of formats) options.outDir = '\''release-'\'' + options.name; export default { pack: process.env.PACK_TARGET === '\''cli'\'' ? cli : library };'`


## `vpt mkdir -p public`


## `vpt write-file public/resource.txt 'library resource
'`


## `vp install --ignore-scripts`


## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`


## `PACK_TARGET=library vp pack`


## `vpt stat-file release-library/library.mjs --assert file`

```
release-library/library.mjs: file
```

## `vpt print-file release-library/public/resource.txt`

```
library resource
```

## `vpt stat-file library-out cli-out release-cli --assert missing`

```
library-out: missing
cli-out: missing
release-cli: missing
```

## `vpt rm -r release-library`


## `PACK_TARGET=cli vp pack`


## `vpt stat-file release-cli/cli.mjs --assert file`

```
release-cli/cli.mjs: file
```

## `vpt stat-file library-out cli-out release-library --assert missing`

```
library-out: missing
cli-out: missing
release-library: missing
```

## `pnpm run bundle`


## `vpt stat-file library-out/library.mjs --assert file`

```
library-out/library.mjs: file
```

## `vpt print-file library-out/public/resource.txt`

```
library resource
```
