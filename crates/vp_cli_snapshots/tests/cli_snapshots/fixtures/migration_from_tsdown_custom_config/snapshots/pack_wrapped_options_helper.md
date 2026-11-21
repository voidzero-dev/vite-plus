# pack_wrapped_options_helper

The same converted pack object reaches a helper that updates entry and outDir.

## `vpt json-edit package.json devDependencies.vite-plus ^1.7.0`


## `vpt json-edit package.json devDependencies.tsdown 0.22.14`


## `vpt json-edit package.json scripts.bundle 'tsdown -c tsdown.config.ts'`


## `vpt write-file tsdown.config.ts 'export default { entry: ['\''src/library.ts'\''], outDir: '\''library-out'\'', publicDir: '\''public'\'' };
'`


## `vpt write-file src/extra.ts 'export const extra = '\''helper marker'\'';
'`


## `vpt write-file vite.config.ts 'import base from '\''./tsdown.config.ts'\'';
function configure({ options }: { options: typeof base }) {
  options.entry.push('\''src/extra.ts'\'');
  options.outDir = '\''release-out'\'';
}
configure({ options: base });
export default { pack: base };
'`


## `vpt mkdir -p public`


## `vpt write-file public/resource.txt 'library resource
'`


## `vp install --ignore-scripts`


## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`


## `vp pack`


## `vpt stat-file release-out/library.mjs release-out/extra.mjs --assert file`

```
release-out/library.mjs: file
release-out/extra.mjs: file
```

## `vpt print-file release-out/public/resource.txt`

```
library resource
```

## `vpt stat-file library-out --assert missing`

```
library-out: missing
```

## `pnpm run bundle`


## `vpt stat-file library-out/library.mjs --assert file`

```
library-out/library.mjs: file
```

## `vpt stat-file library-out/extra.mjs --assert missing`

```
library-out/extra.mjs: missing
```

## `vpt print-file library-out/public/resource.txt`

```
library resource
```
