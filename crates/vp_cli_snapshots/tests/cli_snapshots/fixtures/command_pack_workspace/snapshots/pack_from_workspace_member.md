# pack_from_workspace_member

## `vpt cp serial.config.ts vite.config.ts`

```
```

## `cd packages/a && vp pack`


## `node verify.mjs packages/a/dist/index.mjs`

```
```

## `vpt stat-file dist/index.mjs packages/b/dist/index.mjs --assert missing`

```
dist/index.mjs: missing
packages/b/dist/index.mjs: missing
```
