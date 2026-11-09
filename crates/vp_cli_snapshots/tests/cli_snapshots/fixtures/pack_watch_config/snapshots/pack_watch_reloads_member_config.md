# pack_watch_reloads_member_config

## `vpt mkdir -p packages/a`

```
```

## `vpt cp index.ts packages/a/index.ts`

```
```

## `vpt write-file packages/a/tsdown.config.mjs 'export default { minify: false }'`

```
```

## `node verify.mjs member`

```
pack --watch applied the updated minify option
```
