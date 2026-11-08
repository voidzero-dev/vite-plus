# command_pack_workspace

## `vp pack`

use the workspace and exclusions from vite.config.ts


## `vpt stat-file dist/index.mjs packages/b/dist/index.mjs --assert missing`

```
dist/index.mjs: missing
packages/b/dist/index.mjs: missing
```

## `node verify.mjs packages/a/dist/index.mjs`

```
```

## `vpt rm -rf packages/a/dist`

```
```

## `vp pack --workspace packages/*`

build both members with their own entries


## `vpt stat-file dist/index.mjs --assert missing`

```
dist/index.mjs: missing
```

## `node verify.mjs packages/a/dist/index.mjs packages/b/dist/index.mjs`

```
```

## `vpt rm -rf packages/a/dist packages/b/dist`

```
```

## `vp pack --workspace packages/* --filter b`

filter the expanded members by package name


## `vpt stat-file dist/index.mjs packages/a/dist/index.mjs --assert missing`

```
dist/index.mjs: missing
packages/a/dist/index.mjs: missing
```

## `node verify.mjs packages/b/dist/index.mjs`

```
```

## `vpt rm -rf packages/b/dist`

```
```

## `vpt cp serial.config.ts vite.config.ts`

```
```

## `vpt cp packages/b/member.ts packages/b/index.ts`

```
```

## `vpt write-file packages/b/tsdown.config.mjs 'export default { outDir: '\''from-member'\'' }'`

```
```

## `vp pack --cwd . --concurrency 1 --out-dir from-cli`

serialize member builds and give explicit CLI options precedence


## `vpt stat-file packages/b/from-member --assert missing`

```
packages/b/from-member: missing
```

## `node verify.mjs packages/a/from-cli/index.mjs packages/b/from-cli/index.mjs`

```
```

## `vpt cp disabled.config.ts vite.config.ts`

```
```

## `vpt write-file packages/b/tsdown.config.mjs 'throw new Error('\''unexpected member config'\'')'`

```
```

## `vp pack`

skip member configuration but build in each member directory


## `vpt stat-file dist/index.mjs --assert missing`

```
dist/index.mjs: missing
```

## `node verify.mjs packages/a/dist/index.mjs packages/b/dist/index.mjs`

```
```

## `vpt rm -rf packages/a/dist packages/b/dist`

```
```

## `vpt write-file vite.config.ts 'throw new Error('\''unexpected root config'\'')'`

```
```

## `vp pack index.ts --no-config --workspace packages/*`

skip both root and member configuration with --no-config


## `vpt stat-file dist/index.mjs --assert missing`

```
dist/index.mjs: missing
```

## `node verify.mjs packages/a/dist/index.mjs packages/b/dist/index.mjs`

```
```
