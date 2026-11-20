# pack_mutated_import

An assignment to the imported pack options controls the output directory after migration.

## `vpt json-edit package.json devDependencies.vite-plus ^1.7.0`


## `vpt write-file tsdown.config.ts 'export default { entry: '\''src/library.ts'\'', outDir: '\''library-out'\'', publicDir: '\''public'\'' };'`


## `vpt write-file vite.config.ts 'import pack from '\''./tsdown.config.ts'\''; pack.outDir = '\''release-out'\''; export default { pack, build: { lib: { entry: pack.entry } } };'`


## `vp install --ignore-scripts`


## `vp pack`


## `vpt stat-file release-out/library.mjs --assert file`


## `vpt rm -r release-out`


## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`


## `vp pack`


## `vpt print-file release-out/library.mjs`

```
// src/library.ts
const answer = 42;
export { answer };
```

## `vpt stat-file library-out/library.mjs vite-pack.config.ts --assert missing`

```
library-out/library.mjs: missing
vite-pack.config.ts: missing
```
