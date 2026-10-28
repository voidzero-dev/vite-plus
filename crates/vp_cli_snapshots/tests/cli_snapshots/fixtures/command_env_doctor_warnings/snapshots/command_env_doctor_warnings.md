# command_env_doctor_warnings

Warnings describe setups that still work, so doctor names them in the summary and exits with 0.

## `vpt write-file $HOME/.profile '. "$HOME/.vite-plus/env"'`

source the env file so the output has no platform-specific IDE guidance


## `VP_NODE_VERSION=20.20.2 vp env doctor node`

```
VITE+ - The Unified Toolchain for the Web

Installation
  ✓ Bin dir           ~/.vite-plus/bin
  ✓ Data dir          ~/.vite-plus
  ✓ Cache dir         ~/.vite-plus/cache
  ✓ Config dir        ~/.vite-plus
  ✓ State dir         ~/.vite-plus
  ✓ Shims             node

Configuration
  ✓ Node.js           managed mode
  ✓ IDE integration   env sourced in ~/.profile
  ⚠ Session override  VP_NODE_VERSION=20.20.2
  note: Overrides all file-based resolution.
  note: Run 'vp env use --unset' to remove.

PATH
  ✓ vp                ~/.vite-plus/bin/vp
  ✓ Shim dir          ~/.vite-plus/bin
  ✓ node              ~/.vite-plus/bin/node (vp shim)

Node.js Resolution
    Directory         <workspace>
    Source            VP_NODE_VERSION
    Version           20.20.2
  ⚠ Node binary       not installed
  note: Version will be downloaded on first use.

⚠ Some warnings found.
```
