# shim_corepack_enable_install_directory

## `vpt mkdir -p home/js_runtime/node/22.18.0/bin`

Isolated VP_HOME with a fake managed Node runtime layout


## `vpt write-file .node-version '22.18.0
'`

Project Node.js version


## `vpt write-file home/js_runtime/node/22.18.0/bin/node '#'\!'/bin/sh
echo fake-node
'`

Fake node binary


## `vpt chmod +x home/js_runtime/node/22.18.0/bin/node`


## `vpt cp fake-corepack.sh home/js_runtime/node/22.18.0/bin/corepack`

Fake bundled corepack that echoes its args


## `vpt chmod +x home/js_runtime/node/22.18.0/bin/corepack`


## `VP_HOME=${workspace}/home vp env setup`

Create shims in the isolated home


## `VP_HOME=${workspace}/home PATH=${workspace}/home/bin:${PATH} corepack use pnpm@10`

Non-link commands run unchanged

```
corepack use pnpm@10
```

## `VP_HOME=${workspace}/home PATH=${workspace}/home/bin:${PATH} corepack enable --install-directory /tmp/custom-dir`

Explicit --install-directory is respected, clobbered npm shim is restored

```
corepack enable --install-directory /tmp/custom-dir
warn: 'npm' is managed by Vite+ and was restored. Vite+ already resolves 'npm' per project, so corepack does not need to manage it.
```

## `VP_HOME=${workspace}/home PATH=${workspace}/home/bin:${PATH} corepack enable`

--install-directory defaults to VP_HOME/bin

```
corepack enable --install-directory <root>/home/bin
warn: 'npm' is managed by Vite+ and was restored. Vite+ already resolves 'npm' per project, so corepack does not need to manage it.
```

## `vpt stat-file home/bin/npm --assert symlink`

Vite+ owns the npm shim

```
home/bin/npm: symlink
```
