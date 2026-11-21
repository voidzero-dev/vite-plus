# known_config_suffixed_output

A literal suffix on a quoted output directory leaves the explicit tsdown config selected.

## `vpt json-edit package.json scripts.bundle 'tsdown --config tsdown.custom.config.ts --out-dir "$OUT_DIR/release"'`


## `vpt json-edit packages/legacy-pack/package.json scripts.build tsdown`


## `vpt json-edit packages/legacy-pack/package.json devDependencies.tsdown catalog:build`


## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`


## `vpt print-file package.json packages/legacy-pack/package.json`

```
{
  "devDependencies": {
    "tsdown": "catalog:build",
    "vite-plus": "catalog:"
  },
  "name": "migration-from-tsdown-mixed-workspace",
  "packageManager": "pnpm@12.11.2",
  "private": true,
  "scripts": {
    "bundle": "tsdown --config tsdown.custom.config.ts --out-dir \"$OUT_DIR/release\""
  },
  "type": "module"
}
{
  "devDependencies": {},
  "name": "legacy-pack",
  "private": true,
  "scripts": {
    "build": "vp pack"
  },
  "type": "module"
}
```

## `pnpm install --ignore-scripts`


## `OUT_DIR=release-artifacts pnpm run bundle`


## `vpt stat-file release-artifacts/release/root-library.mjs --assert file`

```
release-artifacts/release/root-library.mjs: file
```

## `cd packages/legacy-pack && pnpm run build`


## `vpt stat-file packages/legacy-pack/lib-out/library.mjs --assert file`

```
packages/legacy-pack/lib-out/library.mjs: file
```
