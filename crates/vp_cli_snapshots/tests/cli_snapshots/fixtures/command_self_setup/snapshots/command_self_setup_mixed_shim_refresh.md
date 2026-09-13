# command_self_setup_mixed_shim_refresh

## `vpt mkdir -p external home/bin user-bin`


## `vpt cp $VP_HOME/bin/vp external/vp`


## `vpt chmod +x external/vp`


## `vpt write-file home/bin/node old-node-shim`


## `vpt write-file home/bin/npm old-npm-shim`


## `vpt write-file home/bin/pnpm old-pnpm-shim`


## `vpt write-file home/bin/pnpx old-pnpx-shim`


## `vpt write-file user-bin/node user-node-shim`


## `vpt write-file user-bin/pnpm user-pnpm-shim`


## `VP_HOME=${workspace}/home VP_VERSION=mixed-shims VP_NODE_MANAGER=no VP_PM_MANAGER=no VP_PNPM_MANAGER=yes PATH=${workspace}/user-bin${PATH_SEPARATOR}${PATH} ./external/vp`

Installation refreshes every Vite+ shim regardless of management preferences, leaving user tools elsewhere on PATH untouched


## `vpt stat-file home/bin/node --assert symlink`

```
home/bin/node: symlink
```

## `vpt stat-file home/bin/npm --assert symlink`

```
home/bin/npm: symlink
```

## `vpt stat-file home/bin/pnpm --assert symlink`

```
home/bin/pnpm: symlink
```

## `vpt stat-file home/bin/pnpx --assert symlink`

```
home/bin/pnpx: symlink
```

## `vpt print-file user-bin/node`

```
user-node-shim
```

## `vpt print-file user-bin/pnpm`

```
user-pnpm-shim
```

## `vpt print-file home/config.json`

```
{
  "nodeShimMode": "system_first",
  "packageManagerShimModes": {
    "bun": "system_first",
    "npm": "system_first",
    "pnpm": "managed",
    "yarn": "system_first"
  }
}
```
