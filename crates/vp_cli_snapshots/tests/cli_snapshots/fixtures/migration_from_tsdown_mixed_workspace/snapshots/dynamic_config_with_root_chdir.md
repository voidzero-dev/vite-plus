# dynamic_config_with_root_chdir

The root script builds a package that has a default tsdown config and no build script. Keep the original config available to standalone tsdown.

## `vpt json-edit package.json scripts.bundle 'export TSDOWN_CONFIG=tsdown.custom.config.ts && tsdown --config "$TSDOWN_CONFIG"'`


## `vpt json-edit package.json scripts.bundleChild 'vp -C packages/legacy-pack pack'`


## `vpt write-file packages/legacy-pack/package.json '{"name":"legacy-pack","private":true,"type":"module"}'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`


## `vp run bundle`


## `vpt print-file custom-out/root-library.mjs`

```
// src/root-library.ts
const rootAnswer = 42;
export { rootAnswer };
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

## `vpt stat-file packages/legacy-pack/tsdown.config.json --assert file`

```
packages/legacy-pack/tsdown.config.json: file
```

## `vpt rm -r packages/legacy-pack/lib-out`


## `cd packages/legacy-pack && vp pack`


## `vpt stat-file packages/legacy-pack/lib-out/library.mjs --assert file`

```
packages/legacy-pack/lib-out/library.mjs: file
```
