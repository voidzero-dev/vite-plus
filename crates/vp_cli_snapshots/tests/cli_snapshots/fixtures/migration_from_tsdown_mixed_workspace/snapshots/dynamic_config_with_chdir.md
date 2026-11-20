# dynamic_config_with_chdir

## `vpt json-edit package.json scripts.bundle 'export TSDOWN_CONFIG=tsdown.custom.config.ts && tsdown --config "$TSDOWN_CONFIG"'`


## `vpt json-edit packages/legacy-pack/package.json scripts.build 'vp -C . pack'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`


## `vp run bundle`


## `vpt print-file custom-out/root-library.mjs`

```
// src/root-library.ts
const rootAnswer = 42;
export { rootAnswer };
```

## `cd packages/legacy-pack && vp run build`


## `vpt stat-file packages/legacy-pack/lib-out/library.mjs --assert file`

```
packages/legacy-pack/lib-out/library.mjs: file
```

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

## `vpt stat-file packages/legacy-pack/tsdown.config.json --assert file`

```
packages/legacy-pack/tsdown.config.json: file
```
