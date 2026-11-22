# pack_nullish_entry_alias

Entry aliases with both fallback operators keep the appended entries and library resource.

## `vpt json-edit package.json devDependencies.vite-plus ^1.7.0`


## `vpt json-edit package.json devDependencies.tsdown 0.22.14`


## `vpt write-file src/cli.ts 'export const cli = true;'`


## `vpt write-file src/tool.ts 'export const tool = true;'`


## `vpt write-file tsdown.config.ts 'export default { entry: ['\''src/library.ts'\''], outDir: '\''library-out'\'', publicDir: '\''public'\'' };'`


## `vpt write-file vite.config.ts 'import base from '\''./tsdown.config.ts'\''; const entries = base.entry ?? []; entries.push('\''src/cli.ts'\''); const tools = base.entry || []; tools.push('\''src/tool.ts'\''); export default { pack: base };'`


## `vpt mkdir -p public`


## `vpt write-file public/resource.txt 'library resource
'`


## `vp install --ignore-scripts`


## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`


## `vp pack`


## `vpt stat-file library-out/library.mjs library-out/cli.mjs library-out/tool.mjs --assert file`

```
library-out/library.mjs: file
library-out/cli.mjs: file
library-out/tool.mjs: file
```

## `vpt print-file library-out/public/resource.txt`

```
library resource
```
