# command_pack_workspace

## `vp pack`

use the workspace and exclusions from vite.config.ts


## `node verify.mjs a`

```
```

## `vp pack --workspace packages/*`

build both members with their own entries


## `node verify.mjs a b`

```
```

## `vp pack --workspace packages/* --filter b`

filter the expanded members by package name


## `node verify.mjs b`

```
```
