# native_loader

## `vpt json-edit package.json devDependencies.vite-plus ^1.7.0`


## `vpt write-file tsdown.config.ts 'import { defineConfig } from '\''tsdown'\''; export default defineConfig({ entry: ['\''src/library.ts'\''] });'`


## `vpt write-file vite.config.ts 'import { defineConfig } from '\''vite-plus'\''; import pack from '\''./tsdown.config.ts'\''; export default defineConfig({ pack });'`


## `vpt write-file index.html '<main>Native config</main>'`


## `vp install --ignore-scripts`


## `vp build --configLoader native`


## `vpt rm -r dist`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`


## `vpt stat-file vite-pack.config.ts --assert file`

```
vite-pack.config.ts: file
```

## `vp build --configLoader native`


## `vpt print-file dist/index.html`

```
<main>Native config</main>
```
