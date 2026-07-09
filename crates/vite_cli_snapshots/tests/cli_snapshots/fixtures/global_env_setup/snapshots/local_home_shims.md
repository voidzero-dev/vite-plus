# local_home_shims

The local flavor installs the case-local standalone layout, then exposes the case-local package bin.

## `vp --version`

vp resolves from the case-local package bin


## `node --version`

node resolves from the case home shim


## `vpt stat-file ../home/.vite-plus/current/bin/vp ../home/.vite-plus/current/node_modules/vite-plus/bin/vp ../home/.vite-plus/current/node_modules/vite-plus/bin/vpr ../home/.vite-plus/current/node_modules/vite-plus/bin/oxfmt ../home/.vite-plus/current/node_modules/vite-plus/bin/oxlint ../home/.vite-plus/bin/vp ../home/.vite-plus/bin/vpr ../home/.vite-plus/bin/vpx ../home/.vite-plus/bin/node --assert file`

vp env setup created shims and the local package bin is inside the case home

```
../home/.vite-plus/current/bin/vp: file
../home/.vite-plus/current/node_modules/vite-plus/bin/vp: file
../home/.vite-plus/current/node_modules/vite-plus/bin/vpr: file
../home/.vite-plus/current/node_modules/vite-plus/bin/oxfmt: file
../home/.vite-plus/current/node_modules/vite-plus/bin/oxlint: file
../home/.vite-plus/bin/vp: file
../home/.vite-plus/bin/vpr: file
../home/.vite-plus/bin/vpx: file
../home/.vite-plus/bin/node: file
```
