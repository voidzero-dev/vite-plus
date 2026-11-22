# manifest_workspace

## `vpt json-edit package.json tsdown.workspace true`


## `vpt json-edit package.json scripts.build tsdown`


## `vp install --ignore-scripts`


## `vp run build`


## `vpt print-file packages/legacy-pack/lib-out/library.mjs`

```
// src/library.ts
const libraryAnswer = 17;
export { libraryAnswer };
```

## `vpt rm -r packages/legacy-pack/lib-out`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`


## `vpt stat-file packages/legacy-pack/tsdown.config.json --assert file`

```
packages/legacy-pack/tsdown.config.json: file
```

## `vp run build`


## `vpt print-file packages/legacy-pack/lib-out/library.mjs`

```
// src/library.ts
const libraryAnswer = 17;
export { libraryAnswer };
```

## `vpt stat-file packages/legacy-pack/dist/index.mjs --assert missing`

```
packages/legacy-pack/dist/index.mjs: missing
```
