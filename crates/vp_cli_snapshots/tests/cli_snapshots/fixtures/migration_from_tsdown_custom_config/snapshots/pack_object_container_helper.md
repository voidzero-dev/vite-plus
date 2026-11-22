# pack_object_container_helper

Use the output directory set by a helper on the named pack options.

## `vpt json-edit package.json devDependencies.vite-plus ^1.7.0`


## `vpt json-edit package.json devDependencies.tsdown 0.22.14`


## `vpt json-edit package.json scripts.bundle 'tsdown -c tsdown.config.ts'`


## `vpt write-file tsdown.config.ts 'export default { entry: ['\''src/library.ts'\''], outDir: '\''standalone-dist'\'', publicDir: '\''public'\'' };
'`


## `vpt write-file vite.config.ts 'import library from '\''./tsdown.config.ts'\'';
const targets = { library };
function configure(value: { library: typeof library }) {
  value.library.outDir = '\''configured-dist'\'';
}
configure(targets);
export default { pack: library };
'`


## `vpt mkdir -p public`


## `vpt write-file public/resource.txt 'library resource
'`


## `vp install --ignore-scripts`


## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`


## `vp pack`


## `vpt stat-file configured-dist/library.mjs configured-dist/public/resource.txt --assert file`

```
configured-dist/library.mjs: file
configured-dist/public/resource.txt: file
```

## `vpt stat-file standalone-dist --assert missing`

```
standalone-dist: missing
```

## `pnpm run bundle`


## `vpt stat-file standalone-dist/library.mjs standalone-dist/public/resource.txt --assert file`

```
standalone-dist/library.mjs: file
standalone-dist/public/resource.txt: file
```
