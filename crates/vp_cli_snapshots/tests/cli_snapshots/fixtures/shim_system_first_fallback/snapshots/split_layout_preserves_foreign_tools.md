# split_layout_preserves_foreign_tools

## `vpt mkdir -p data/current`


## `vpt cp -r $VP_HOME/current/bin data/current/bin`

Copy the installed CLI and its setup marker into the split data root.


## `node setup-split-layout.cjs`


## `HOME=${workspace}/user VP_HOME= VP_BIN_DIR=${workspace}/shared-bin VP_DATA_DIR=${workspace}/data VP_CACHE_DIR=${workspace}/cache XDG_CONFIG_HOME=${workspace}/config PATH=${workspace}/data/current/bin${PATH_SEPARATOR}${PATH} vp env setup --refresh`


## `node list-shims.cjs shared-bin data/fallback-bin`

Setup preserves the existing executable in the shared bin.

```
shared-bin: bun, bunx, node, npm, npx, pnpm, pnpx, vpr, vpx, yarn, yarnpkg
data/fallback-bin: (empty)
```

## `HOME=${workspace}/user VP_HOME= VP_BIN_DIR=${workspace}/shared-bin VP_DATA_DIR=${workspace}/data VP_CACHE_DIR=${workspace}/cache XDG_CONFIG_HOME=${workspace}/config sh split.sh`

The generated shell script puts shared bin first and data-root fallback last.

```
Split shell PATH starts with shared bin and ends with data-root fallback
```

## `PATH=${workspace}/shared-bin${PATH_SEPARATOR}${PATH} node -p process.execPath.includes('shared-bin')`

The foreign Node remains executable.

```
true
```

## `HOME=${workspace}/user VP_HOME= VP_BIN_DIR=${workspace}/shared-bin VP_DATA_DIR=${workspace}/data VP_CACHE_DIR=${workspace}/cache XDG_CONFIG_HOME=${workspace}/config PATH=${workspace}/data/current/bin${PATH_SEPARATOR}${PATH} vp env off node`


## `node list-shims.cjs shared-bin data/fallback-bin`

System-first Node lives under the data root.

```
shared-bin: bun, bunx, node, npm, npx, pnpm, pnpx, vpr, vpx, yarn, yarnpkg
data/fallback-bin: node
```

## `PATH=${workspace}/shared-bin${PATH_SEPARATOR}${PATH} node -p process.execPath.includes('shared-bin')`

The foreign Node remains executable.

```
true
```

## `HOME=${workspace}/user VP_HOME= VP_BIN_DIR=${workspace}/shared-bin VP_DATA_DIR=${workspace}/data VP_CACHE_DIR=${workspace}/cache PATH=${workspace}/data/fallback-bin${PATH_SEPARATOR}${PATH} node -p 'process.env.VP_BIN_DIR === require('\''node:path'\'').resolve('\''shared-bin'\'')'`

The split fallback executes managed Node with the configured main bin root.

```
true
```

## `HOME=${workspace}/user VP_HOME= VP_BIN_DIR=${workspace}/shared-bin VP_DATA_DIR=${workspace}/data VP_CACHE_DIR=${workspace}/cache XDG_CONFIG_HOME=${workspace}/config PATH=${workspace}/data/current/bin${PATH_SEPARATOR}${PATH} vp env setup --refresh`


## `node list-shims.cjs shared-bin data/fallback-bin`

Refresh preserves both placement and the foreign Node.

```
shared-bin: bun, bunx, node, npm, npx, pnpm, pnpx, vpr, vpx, yarn, yarnpkg
data/fallback-bin: node
```

## `PATH=${workspace}/shared-bin${PATH_SEPARATOR}${PATH} node -p process.execPath.includes('shared-bin')`

The foreign Node remains executable.

```
true
```

## `HOME=${workspace}/user VP_HOME= VP_BIN_DIR=${workspace}/shared-bin VP_DATA_DIR=${workspace}/data VP_CACHE_DIR=${workspace}/cache XDG_CONFIG_HOME=${workspace}/config PATH=${workspace}/data/current/bin${PATH_SEPARATOR}${PATH} vp env on node`


## `node list-shims.cjs shared-bin data/fallback-bin`

Managed mode must not overwrite a foreign executable either.

```
shared-bin: bun, bunx, node, npm, npx, pnpm, pnpx, vpr, vpx, yarn, yarnpkg
data/fallback-bin: (empty)
```

## `PATH=${workspace}/shared-bin${PATH_SEPARATOR}${PATH} node -p process.execPath.includes('shared-bin')`

The foreign Node remains executable.

```
true
```
