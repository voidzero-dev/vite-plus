# command_self_setup_mixed_shim_refresh

## `vpt mkdir -p external home/bin other/bin`


## `vpt cp $VP_HOME/bin/vp external/vp`


## `vpt chmod +x external/vp`


## `vpt write-file home/bin/node existing-node`


## `vpt write-file home/bin/npm existing-npm`


## `vpt write-file home/bin/pnpm existing-pnpm`


## `vpt write-file home/bin/pnpx existing-pnpx`


## `VP_HOME=${workspace}/home VP_VERSION=mixed-shims VP_NODE_MANAGER=no VP_PM_MANAGER=no VP_PNPM_MANAGER=yes ./external/vp`

Managing pnpm replaces its existing commands while preserving system-first Node and npm


## `vpt print-file home/bin/node`

```
existing-node
```

## `vpt print-file home/bin/npm`

```
existing-npm
```

## `vpt stat-file home/bin/pnpm --assert symlink`

```
home/bin/pnpm: symlink
```

## `vpt stat-file home/bin/pnpx --assert symlink`

```
home/bin/pnpx: symlink
```

## `vpt write-file other/bin/node existing-node`


## `vpt write-file other/bin/npm existing-npm`


## `VP_HOME=${workspace}/other VP_VERSION=mixed-shims VP_NODE_MANAGER=yes VP_PM_MANAGER=no ./external/vp`

Managing Node preserves an existing system-first npm command


## `vpt stat-file other/bin/node --assert symlink`

```
other/bin/node: symlink
```

## `vpt print-file other/bin/npm`

```
existing-npm
```
