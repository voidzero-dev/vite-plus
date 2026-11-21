# pack_array_entries

A named array enumerated with entries() keeps the original module and resource locations.

## `vpt json-edit package.json devDependencies.vite-plus ^1.7.0`


## `vpt json-edit package.json devDependencies.tsdown 0.22.14`


## `vpt json-edit package.json scripts.bundle 'tsdown -c tsdown.config.ts'`


## `vpt write-file tsdown.config.ts 'export default { name: '\''library'\'', entry: '\''src/library.ts'\'', outDir: '\''library-out'\'', publicDir: '\''public'\'' };'`


## `vpt write-file src/cli.ts 'export const cli = true;'`


## `vpt write-file vite.config.ts 'import library from '\''./tsdown.config.ts'\''; const cli = { name: '\''cli'\'', entry: '\''src/cli.ts'\'', outDir: '\''cli-out'\'' }; const formats = [library, cli]; for (const [index, options] of formats.entries()) options.outDir = '\''release-'\'' + index + '\''-'\'' + options.name; export default { pack: process.env.PACK_TARGET === '\''cli'\'' ? cli : library };'`


## `vpt mkdir -p public`


## `vpt write-file public/resource.txt 'library resource
'`


## `vp install --ignore-scripts`


## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`


## `PACK_TARGET=library vp pack`


## `vpt stat-file release-0-library/library.mjs --assert file`

```
release-0-library/library.mjs: file
```

## `vpt print-file release-0-library/public/resource.txt`

```
library resource
```

## `vpt stat-file library-out cli-out release-1-cli --assert missing`

```
library-out: missing
cli-out: missing
release-1-cli: missing
```

## `vpt rm -r release-0-library`


## `PACK_TARGET=cli vp pack`


## `vpt stat-file release-1-cli/cli.mjs --assert file`

```
release-1-cli/cli.mjs: file
```

## `vpt stat-file library-out cli-out release-0-library --assert missing`

```
library-out: missing
cli-out: missing
release-0-library: missing
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
