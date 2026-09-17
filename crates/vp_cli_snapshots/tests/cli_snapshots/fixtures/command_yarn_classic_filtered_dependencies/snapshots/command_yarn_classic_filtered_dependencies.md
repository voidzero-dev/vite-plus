# command_yarn_classic_filtered_dependencies

## `vp add react --filter @example/app`

Classic filtered add reports an unsupported option instead of invoking foreach

**Exit code:** 1

```
Invalid argument: `--filter` is not supported by Yarn Classic `add`.
```

## `vp install react --filter @example/app`

install with packages uses the same add guard

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

Invalid argument: `--filter` is not supported by Yarn Classic `add`.
```

## `vp remove lodash --filter @example/app`

Classic filtered remove fails before changing dependencies

**Exit code:** 1

```
Invalid argument: `--filter` is not supported by Yarn Classic `remove`.
```

## `vp remove lodash --filter @example/* --filter other --recursive`

recursive remove must not silently discard Classic filters

**Exit code:** 1

```
Invalid argument: `--filter` is not supported by Yarn Classic `remove`.
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
