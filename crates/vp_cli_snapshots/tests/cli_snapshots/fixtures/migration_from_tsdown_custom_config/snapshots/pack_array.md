# pack_array

Convert the retained library config through a local constant and build both bundles with their assets.

## `vpt json-edit package.json devDependencies.vite-plus ^1.7.0`


## `vpt json-edit package.json devDependencies.tsdown 0.22.14`


## `vpt write-file tsdown.config.ts 'import { defineConfig } from '\''tsdown'\''; export default defineConfig({ entry: '\''src/library.ts'\'', outDir: '\''library-out'\'', publicDir: '\''public'\'', dts: true });'`


## `vpt write-file src/cli.ts 'export const cli = true;'`


## `vpt write-file cli.config.ts 'export default { entry: '\''src/cli.ts'\'', outDir: '\''cli-out'\'', copy: '\''cli-resources'\'' };'`


## `vpt write-file vite.config.ts 'import libraryConfig from '\''./tsdown.config.ts'\''; import cliConfig from '\''./cli.config.ts'\''; const library = { ...libraryConfig, dts: false }; export default { pack: [library, cliConfig] };'`


## `vpt mkdir -p public cli-resources`


## `vpt write-file public/asset.txt 'library asset
'`


## `vpt write-file cli-resources/asset.txt 'cli asset
'`


## `vp install --ignore-scripts`


## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`


## `vp run bundle`


## `vpt stat-file lib-out/library.mjs --assert file`


## `vp pack`


## `vpt stat-file library-out/library.mjs --assert file`


## `vpt stat-file library-out/library.d.mts --assert missing`


## `vpt stat-file cli-out/cli.mjs --assert file`


## `vpt print-file library-out/public/asset.txt cli-out/cli-resources/asset.txt`

```
library asset
cli asset
```
