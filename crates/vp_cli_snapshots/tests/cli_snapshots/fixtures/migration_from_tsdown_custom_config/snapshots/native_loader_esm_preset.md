# native_loader_esm_preset

The native loader can import a converted ESM preset in a CommonJS project.

## `vpt json-edit package.json type commonjs`


## `vpt json-edit package.json scripts.bundle 'tsdown -c tsdown.config.js'`


## `vpt json-edit package.json devDependencies.vite-plus ^1.7.0`


## `vpt json-edit package.json devDependencies.tsdown 0.22.14`


## `vp install --ignore-scripts`


## `vpt mkdir -p node_modules/build-preset public`


## `vpt write-file node_modules/build-preset/package.json '{"name":"build-preset","version":"1.0.0","type":"module"}'`


## `vpt write-file node_modules/build-preset/pack.js 'export default { entry: '\''src/library.ts'\'', outDir: '\''library-out'\'', publicDir: '\''public'\'' };'`


## `node -e 'require('\''node:fs'\'').symlinkSync('\''node_modules/build-preset/pack.js'\'', '\''tsdown.config.js'\'', '\''file'\'')'`


## `vpt write-file vite.config.mts 'import pack from '\''./tsdown.config.js'\''; export default { pack };'`


## `vpt write-file index.html '<main>Native config</main>'`


## `vpt write-file public/asset.txt 'library asset
'`


## `vp build --configLoader native`


## `vpt rm -r dist`


## `VP_SKIP_INSTALL=1 vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`


## `vpt stat-file tsdown.config.js --assert symlink`

```
tsdown.config.js: symlink
```

## `vpt stat-file vite-pack.config.mjs --assert file`

```
vite-pack.config.mjs: file
```

## `vp build --configLoader native`


## `vpt print-file dist/index.html`

```
<main>Native config</main>
```

## `vp pack`


## `vpt print-file library-out/public/asset.txt`

```
library asset
```
