# command_yarn_classic_filtered_dependencies

## `vp add react --filter @example/app`

Classic filtered add reports an unsupported option instead of invoking foreach

**Exit code:** 1

```
yarn < 2 does not support --filter.
```

## `vp install react --filter @example/app`

install with packages uses the same add guard

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

yarn < 2 does not support --filter.
```

## `vp remove lodash --filter @example/app --filter other`

Classic remove rejects multiple filters before changing dependencies

**Exit code:** 1

```
yarn < 2 does not support multiple --filter options.
```

## `vp remove lodash --filter @example/* --recursive`

recursive remove is rejected before the Classic filter translation

**Exit code:** 1

```
yarn < 2 does not support --recursive.
```

## `vpt print-file package.json packages/web/package.json`

root and workspace manifests remain unchanged

```
{
  "name": "command-yarn-classic-filtered-dependencies",
  "private": true,
  "packageManager": "yarn@1.22.22",
  "workspaces": ["packages/*"],
  "dependencies": {
    "lodash": "4.17.21"
  }
}
{
  "name": "@example/app",
  "version": "1.0.0",
  "dependencies": {
    "lodash": "4.17.21"
  }
}
```

## `vpt stat-file yarn.lock --assert missing`

```
yarn.lock: missing
```

## `vpt stat-file node_modules --assert missing`

```
node_modules: missing
```
