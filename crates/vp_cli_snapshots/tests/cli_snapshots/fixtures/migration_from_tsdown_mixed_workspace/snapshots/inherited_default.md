# inherited_default

## `vpt json-edit package.json scripts.bundle 'vp pack'`


## `vpt rm packages/legacy-pack/tsdown.config.json`


## `vpt write-file tsdown.config.json '{"entry":["src/library.ts"],"outDir":"inherited-out","dts":false}'`


## `vpt json-edit packages/legacy-pack/package.json scripts.build tsdown`


## `vpt json-edit packages/legacy-pack/package.json scripts.release 'tsdown -c release.config.ts'`


## `vpt write-file packages/legacy-pack/release.config.ts 'export default { entry: ['\''src/library.ts'\''], dts: false };'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`


## `vpt stat-file tsdown.config.json --assert file`

```
tsdown.config.json: file
```

## `cd packages/legacy-pack && vp run build`


## `vpt print-file packages/legacy-pack/inherited-out/library.mjs`

```
// src/library.ts
const libraryAnswer = 17;
export { libraryAnswer };
```

## `vpt stat-file packages/legacy-pack/dist/index.mjs --assert missing`

```
packages/legacy-pack/dist/index.mjs: missing
```
