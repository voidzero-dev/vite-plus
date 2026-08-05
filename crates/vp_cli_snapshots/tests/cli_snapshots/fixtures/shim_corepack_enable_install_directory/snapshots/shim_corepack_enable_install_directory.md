# shim_corepack_enable_install_directory

## `vpt mkdir -p home/.vite-plus/js_runtime/node/22.18.0/bin home/.vite-plus/current/bin home/.vite-plus/bin`

Isolated legacy install layout with a fake managed Node runtime


## `vpt cp $VP_HOME/current/bin/vp home/.vite-plus/current/bin/vp`

The isolated install's vp binary


## `vpt cp $VP_HOME/current/bin/vp home/.vite-plus/bin/vp`

Marks the layout as a legacy install for detection


## `vpt chmod +x home/.vite-plus/current/bin/vp`


## `vpt chmod +x home/.vite-plus/bin/vp`


## `vpt write-file .node-version '22.18.0
'`

Project Node.js version


## `vpt write-file home/.vite-plus/js_runtime/node/22.18.0/bin/node '#'\!'/bin/sh
echo fake-node
'`

Fake node binary


## `vpt chmod +x home/.vite-plus/js_runtime/node/22.18.0/bin/node`


## `vpt cp fake-corepack.sh home/.vite-plus/js_runtime/node/22.18.0/bin/corepack`

Fake bundled corepack that echoes its args


## `vpt chmod +x home/.vite-plus/js_runtime/node/22.18.0/bin/corepack`


## `./home/.vite-plus/current/bin/vp env setup`

Create shims in the isolated install (self-located, no VP_HOME)


## `PATH=${workspace}/home/.vite-plus/bin:${PATH} corepack use pnpm@10`

Non-link commands run unchanged

```
corepack use pnpm@10
```

## `PATH=${workspace}/home/.vite-plus/bin:${PATH} corepack enable --install-directory /tmp/custom-dir`

Explicit --install-directory is respected, clobbered npm shim is restored

```
corepack enable --install-directory /tmp/custom-dir
```

## `PATH=${workspace}/home/.vite-plus/bin:${PATH} corepack enable`

--install-directory defaults to the install's bin dir

```
corepack enable --install-directory <root>/home/.vite-plus/bin
```

## `vpt stat-file home/.vite-plus/bin/npm --assert symlink`

Vite+ owns the npm shim

```
home/.vite-plus/bin/npm: symlink
```
