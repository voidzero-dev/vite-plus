# global_home_shims

The global flavor installs the case-local standalone layout and only exposes VP_HOME/bin tools.

## `vp --version`

vp resolves from the case home shim


## `node --version`

node resolves from the case home shim


## `corepack --version`

any Vite+-managed shim can run without a runner allowlist


## `vpt stat-file ../home/.vite-plus/current/bin/vp ../home/.vite-plus/current/node_modules/vite-plus/bin/vp ../home/.vite-plus/bin/vp ../home/.vite-plus/bin/vpr ../home/.vite-plus/bin/vpx ../home/.vite-plus/bin/node ../home/.vite-plus/bin/corepack --assert file`

vp env setup created the shims inside the case home

```
../home/.vite-plus/current/bin/vp: file
../home/.vite-plus/current/node_modules/vite-plus/bin/vp: file
../home/.vite-plus/bin/vp: file
../home/.vite-plus/bin/vpr: file
../home/.vite-plus/bin/vpx: file
../home/.vite-plus/bin/node: file
../home/.vite-plus/bin/corepack: file
```
