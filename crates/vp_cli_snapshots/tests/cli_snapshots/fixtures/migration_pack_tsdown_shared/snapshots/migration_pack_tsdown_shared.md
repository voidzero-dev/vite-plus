# migration_pack_tsdown_shared

A workspace member can keep using a linked tsdown config and build with its converted pack config.

## `node create-config-link.mjs`


## `vp migrate --no-interactive --no-agent --no-editor --no-hooks`


## `vpt stat-file packages/lib/tsdown.config.ts --assert symlink`

```
packages/lib/tsdown.config.ts: symlink
```

## `vpt stat-file packages/lib/vite-pack.config.ts --assert symlink`

```
packages/lib/vite-pack.config.ts: symlink
```

## `cd packages/lib && pnpm run release`


## `vpt stat-file packages/lib/shared-out/lib.mjs --assert file`

```
packages/lib/shared-out/lib.mjs: file
```

## `vpt rm -r packages/lib/shared-out`


## `cd packages/lib && vp pack`


## `vpt stat-file packages/lib/shared-out/lib.mjs --assert file`

```
packages/lib/shared-out/lib.mjs: file
```

## `vpt print-file packages/lib/shared-out/public/asset.txt`

```
Shared pack asset
```

## `cd packages/lib && vp build`


## `vpt stat-file packages/lib/dist/index.html --assert file`

```
packages/lib/dist/index.html: file
```
