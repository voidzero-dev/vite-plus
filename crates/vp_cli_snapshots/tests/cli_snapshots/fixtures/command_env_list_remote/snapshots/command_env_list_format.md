# command_env_list_format

## `vpt mkdir -p $VP_HOME/js_runtime/node/22.11.0`


## `vpt write-file $VP_HOME/package_manager/pnpm/10.18.0/pnpm/bin/pnpm '#'\!'/bin/sh
'`


## `vpt write-file $VP_HOME/package_manager/pnpm/10.18.0/pnpm/bin/pnpx '#'\!'/bin/sh
'`


## `vp env default 22.11.0 pnpm@10.18.0`


## `vp env list node`

Installed Node.js versions retain their interactive formatting

```
VITE+ - The Unified Toolchain for the Web

Node.js
  \x1b[94m* <version> \x1b[2mcurrent default

\x1b[2mnote: Run `vp env clean` to free disk space from unused managed runtimes and package manager caches.
```

## `vp env list pnpm`

Installed package-manager versions use the same interactive formatting

```
VITE+ - The Unified Toolchain for the Web

pnpm
  \x1b[94m* 10.18.0 \x1b[2mcurrent default

\x1b[2mnote: Run `vp env clean` to free disk space from unused managed runtimes and package manager caches.
```
