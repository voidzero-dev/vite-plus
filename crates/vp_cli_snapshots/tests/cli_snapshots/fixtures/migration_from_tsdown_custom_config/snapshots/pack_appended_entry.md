# pack_appended_entry

Vite can append an entry through an alias and still build both entries after migration.

## `vpt json-edit package.json devDependencies.vite-plus ^1.7.0`


## `vpt json-edit package.json devDependencies.tsdown 0.22.14`


## `vpt write-file src/cli.ts 'export const cli = true;'`


## `vpt write-file tsdown.config.ts 'export default { entry: ['\''src/library.ts'\''], outDir: '\''library-out'\'' };'`


## `vpt write-file vite.config.ts 'import pack from '\''./tsdown.config.ts'\''; const entries = pack.entry; entries.push('\''src/cli.ts'\''); export default { pack };'`


## `vp install --ignore-scripts`


## `vp pack`


## `vpt stat-file library-out/library.mjs library-out/cli.mjs --assert file`


## `vpt rm -r library-out`


## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`


## `vp pack`


## `vpt print-file library-out/library.mjs library-out/cli.mjs`

```
// src/library.ts
const answer = 42;
export { answer };
// src/cli.ts
const cli = true;
export { cli };
```
