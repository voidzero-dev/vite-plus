# command_env_doctor_broken_config

An unreadable config breaks the shims, so doctor reports it as an error and exits with 1.

## `vpt write-file $HOME/.profile '. "$HOME/.vite-plus/env"'`

source the env file so the output has no platform-specific IDE guidance


## `vpt write-file $VP_HOME/config.json '{ not json'`


## `vp env doctor`

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

Installation
  ✓ Bin dir           ~/.vite-plus/bin
  ✓ Data dir          ~/.vite-plus
  ✓ Cache dir         ~/.vite-plus/cache (not created yet)
  ✓ Config dir        ~/.vite-plus
  ✓ State dir         ~/.vite-plus

Configuration
  ✗ Config            ~/.vite-plus/config.json (could not read)
  ✓ IDE integration   env sourced in ~/.profile

PATH
  ✓ vp                ~/.vite-plus/bin/vp
  ✓ Shim dir          ~/.vite-plus/bin

Node.js Resolution
    Directory         <workspace>
  ✗ Resolution        ~/.vite-plus/config.json (could not read)

Package Manager Resolution
    Package manager   not selected

✗ Some issues found.
```
