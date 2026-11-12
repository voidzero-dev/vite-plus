# command_self_setup_env_forbidden_in_place

## `vpt rm $VP_HOME/current/bin/.vp-setup-complete`

An interrupted upgrade left the active installation unmarked


## `vpt write-file $VP_HOME/current/bin/vp-install-lock.yaml 'release lockfile
'`

The release lockfile is present, so setup would restore it into the installation


## `vpt write-file $VP_HOME/current/node_modules/.vp-install-complete 'stale receipt
'`

A stale receipt makes setup reinstall the dependencies


## `vpt write-file $VP_HOME/current/pnpm-lock.yaml 'existing lockfile
'`


## `VP_ENV_FORBIDDEN=1 PATH=${VP_HOME}/bin vp --help`

With only the Vite+ shims on PATH, setup fails before it prepares the lockfile

**Exit code:** 1

```
error: Setup error: VP_ENV_FORBIDDEN=1: Vite+ installs its dependencies with the system 'node', but the Vite+ shim at '<home>/.vite-plus/bin/node' comes first on PATH; run `vp env off node` to move the shims after the system tools
```

## `vpt print-file $VP_HOME/current/node_modules/.vp-install-complete`

```
stale receipt
```

## `vpt print-file $VP_HOME/current/pnpm-lock.yaml`

```
existing lockfile
```

## `vpt stat-file $VP_HOME/current/bin/.vp-setup-complete --assert missing`

```
<home>/.vite-plus/current/bin/.vp-setup-complete: missing
```
