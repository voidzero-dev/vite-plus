# command_self_setup_shell_warning

## `vpt mkdir -p external home user/.bashrc`


## `vpt cp $VP_HOME/bin/vp external/vp`


## `vpt chmod +x external/vp`


## `HOME=${workspace}/user VP_HOME=${workspace}/home VP_SKIP_DEPS_INSTALL=1 VP_VERSION=bootstrap-test VP_NODE_MANAGER=no bash -c './external/vp > setup.log 2>&1'`

An unreadable shell profile warns without preventing installation


## `vpt grep-file setup.log 'Could not configure shell profiles'`

```
setup.log: found "Could not configure shell profiles"
```

## `vpt stat-file home/current/bin/.vp-setup-complete --assert file`

```
home/current/bin/.vp-setup-complete: file
```

## `vpt stat-file user/.bashrc --assert dir`

```
user/.bashrc: dir
```

## `VP_HOME=${workspace}/home ./home/bin/vp --help`

The installed CLI accepts commands after the warning

