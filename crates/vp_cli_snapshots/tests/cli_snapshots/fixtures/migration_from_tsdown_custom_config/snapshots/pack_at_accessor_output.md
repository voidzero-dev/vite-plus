# pack_at_accessor_output

Keep the last build output override in vp pack and retain the standalone tsdown output.

## `vpt json-edit package.json devDependencies.vite-plus ^1.7.0`


## `vpt json-edit package.json devDependencies.tsdown 0.22.14`


## `vpt json-edit package.json scripts.bundle 'tsdown -c tsdown.config.ts'`


## `vpt write-file tsdown.config.ts 'export default [
  { entry: '\''src/library.ts'\'', outDir: '\''library-dist'\'', publicDir: '\''public'\'' },
  { entry: '\''src/cli.ts'\'', outDir: '\''cli-old'\'' },
];
'`


## `vpt write-file vite.config.ts 'import configs from '\''./tsdown.config.ts'\'';
configs.at(-1)'\!'.outDir = '\''cli-dist'\'';
export default { pack: configs };
'`


## `vpt write-file src/cli.ts 'export const cli = true;
'`


## `vpt mkdir -p public`


## `vpt write-file public/resource.txt 'library resource
'`


## `vp install --ignore-scripts`


## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`


## `vp pack`


## `vpt stat-file library-dist/library.mjs cli-dist/cli.mjs --assert file`

```
library-dist/library.mjs: file
cli-dist/cli.mjs: file
```

## `vpt print-file library-dist/public/resource.txt`

```
library resource
```

## `vpt stat-file cli-old --assert missing`

```
cli-old: missing
```

## `vpt rm -r library-dist cli-dist`


## `pnpm run bundle`


## `vpt stat-file library-dist/library.mjs cli-old/cli.mjs --assert file`

```
library-dist/library.mjs: file
cli-old/cli.mjs: file
```

## `vpt print-file library-dist/public/resource.txt`

```
library resource
```

## `vpt stat-file cli-dist --assert missing`

```
cli-dist: missing
```
