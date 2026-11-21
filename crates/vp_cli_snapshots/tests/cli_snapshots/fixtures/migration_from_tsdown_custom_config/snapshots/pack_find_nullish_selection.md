# pack_find_nullish_selection

Include resources from the selected build and from the fallback build.

## `vpt json-edit package.json devDependencies.vite-plus ^1.7.0`


## `vpt json-edit package.json devDependencies.tsdown 0.22.14`


## `vpt json-edit package.json scripts.bundle 'tsdown -c tsdown.config.ts'`


## `vpt write-file tsdown.config.ts 'import { defineConfig } from '\''tsdown'\'';
export default defineConfig([
  { name: '\''library'\'', entry: ['\''src/library.ts'\''], outDir: '\''library-dist'\'', publicDir: '\''public'\'' },
  { name: '\''cli'\'', entry: ['\''src/cli.ts'\''], outDir: '\''cli-dist'\'', publicDir: '\''public'\'' },
]);
'`


## `vpt write-file vite.config.ts 'import configs from '\''./tsdown.config.ts'\'';
const selected = process.env.PACK_TARGET || '\''cli'\'';
export default { pack: configs.find((config) => config.name === selected) ?? configs[0] };
'`


## `vpt write-file src/cli.ts 'export const kind = '\''cli'\'';
'`


## `vpt mkdir -p public`


## `vpt write-file public/resource.txt 'library resource
'`


## `vp install --ignore-scripts`


## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`


## `PACK_TARGET=cli vp pack`


## `vpt stat-file cli-dist/cli.mjs cli-dist/public/resource.txt --assert file`

```
cli-dist/cli.mjs: file
cli-dist/public/resource.txt: file
```

## `vpt stat-file library-dist --assert missing`

```
library-dist: missing
```

## `vpt rm -r cli-dist`


## `PACK_TARGET=missing vp pack`


## `vpt stat-file library-dist/library.mjs library-dist/public/resource.txt --assert file`

```
library-dist/library.mjs: file
library-dist/public/resource.txt: file
```

## `vpt stat-file cli-dist --assert missing`

```
cli-dist: missing
```

## `vpt rm -r library-dist`


## `pnpm run bundle`


## `vpt stat-file library-dist/library.mjs library-dist/public/resource.txt cli-dist/cli.mjs cli-dist/public/resource.txt --assert file`

```
library-dist/library.mjs: file
library-dist/public/resource.txt: file
cli-dist/cli.mjs: file
cli-dist/public/resource.txt: file
```
