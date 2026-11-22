# pack_filtered_for_each

A filtered array contains the same pack options objects the build uses.

## `vpt json-edit package.json devDependencies.vite-plus ^1.7.0`


## `vpt json-edit package.json devDependencies.tsdown 0.22.14`


## `vpt write-file src/browser.ts 'export const browser = true;'`


## `vpt write-file tsdown.config.ts 'export default [{ name: '\''browser'\'', entry: '\''src/browser.ts'\'', outDir: '\''old-browser'\'', publicDir: '\''public'\'' }, { name: '\''library'\'', entry: '\''src/library.ts'\'', outDir: '\''library-out'\'' }];'`


## `vpt write-file vite.config.ts 'import configs from '\''./tsdown.config.ts'\''; configs.filter(config => config.name === '\''browser'\'').forEach(config => { config.outDir = '\''browser-out'\''; }); export default { pack: configs };'`


## `vpt mkdir -p public`


## `vpt write-file public/resource.txt 'browser resource
'`


## `vp install --ignore-scripts`


## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`


## `vp pack`


## `vpt stat-file library-out/library.mjs browser-out/browser.mjs --assert file`

```
library-out/library.mjs: file
browser-out/browser.mjs: file
```

## `vpt print-file browser-out/public/resource.txt`

```
browser resource
```

## `vpt stat-file old-browser --assert missing`

```
old-browser: missing
```
