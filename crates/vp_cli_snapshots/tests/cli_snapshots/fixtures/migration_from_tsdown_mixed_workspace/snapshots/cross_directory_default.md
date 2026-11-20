# cross_directory_default

## `vpt json-edit package.json scripts.bundleChild 'cd packages/legacy-pack && tsdown'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`


## `vpt stat-file packages/legacy-pack/tsdown.config.json --assert file`

```
packages/legacy-pack/tsdown.config.json: file
```

## `vp run bundleChild`


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
