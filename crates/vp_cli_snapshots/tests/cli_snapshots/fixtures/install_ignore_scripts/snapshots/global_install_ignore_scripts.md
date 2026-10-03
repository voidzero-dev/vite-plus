# global_install_ignore_scripts

Managed global install and add skip dependency lifecycle scripts, including by default. The installed binary works in either case.

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

```
VITE+ - The Unified Toolchain for the Web

info: Installing 1 global package with Node.js <version>
✓ Installed scripted-dep 1.0.0
  Bins: scripted-dep
warning: Lifecycle scripts were skipped for: scripted-dep.
To allow them, reinstall with:
  vp install -g ./scripted-dep-1.0.0.tgz --run-scripts
```

## `scripted-dep skipped`

```
postinstall: skipped
```
