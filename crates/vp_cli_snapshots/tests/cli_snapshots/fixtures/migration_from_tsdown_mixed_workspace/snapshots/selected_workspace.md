# selected_workspace

## `vpt write-file tsdown.custom.config.ts 'export default { workspace: true };'`


## `vp install --ignore-scripts`


## `vp run bundle`


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

## `vp run bundle`


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
