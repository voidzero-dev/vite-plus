# pack_typed_helper

A typed helper can copy pack options and keep the library resource.

## `vpt json-edit package.json devDependencies.vite-plus ^1.7.0`


## `vpt json-edit package.json devDependencies.tsdown 0.22.14`


## `vpt json-edit package.json scripts.bundle 'tsdown -c tsdown.config.ts'`


## `vpt write-file tsdown.config.ts 'export default { name: '\''library'\'', entry: '\''src/library.ts'\'', outDir: '\''library-out'\'', publicDir: '\''public'\'' };'`


## `vpt write-file vite.config.ts 'import base from '\''./tsdown.config.ts'\''; const withDefaults = (options: typeof base) => ({ ...options, format: '\''esm'\'' as const }); export default { pack: withDefaults(base) };'`


## `vpt mkdir -p public`


## `vpt write-file public/resource.txt 'library resource
'`


## `vp install --ignore-scripts`


## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`


## `vp pack`


## `vpt stat-file library-out/library.mjs --assert file`

```
library-out/library.mjs: file
```

## `vpt print-file library-out/public/resource.txt`

```
library resource
```

## `vpt rm -r library-out`


## `pnpm run bundle`


## `vpt stat-file library-out/library.mjs --assert file`

```
library-out/library.mjs: file
```

## `vpt print-file library-out/public/resource.txt`

```
library resource
```
