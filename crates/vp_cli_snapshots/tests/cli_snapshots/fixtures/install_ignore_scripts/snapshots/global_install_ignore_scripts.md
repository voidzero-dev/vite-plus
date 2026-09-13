# global_install_ignore_scripts

Managed global install and add skip dependency lifecycle scripts when requested. Without the flag, scripts still run. The installed binary works in either case.

## `npm pack ./scripted-dep --ignore-scripts`


## `vp install -g ./scripted-dep-1.0.0.tgz --ignore-scripts`


## `scripted-dep skipped`

```
postinstall: skipped
```

## `vp remove -g scripted-dep`


## `vp add -g --ignore-scripts ./scripted-dep-1.0.0.tgz`


## `scripted-dep skipped`

```
postinstall: skipped
```

## `vp remove -g scripted-dep`


## `vp install -g ./scripted-dep-1.0.0.tgz`


## `scripted-dep ran`

```
postinstall: ran
```
