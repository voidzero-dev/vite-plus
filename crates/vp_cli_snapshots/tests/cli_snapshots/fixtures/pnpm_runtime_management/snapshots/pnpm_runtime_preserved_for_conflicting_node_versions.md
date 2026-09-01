# pnpm_runtime_preserved_for_conflicting_node_versions

pnpm keeps runtime ownership when Vite+ selects a different Node.js version.

## `vpt write-file package.json '{"name":"pnpm-runtime-management","private":true,"packageManager":"pnpm@11.1.0","devEngines":{"runtime":{"name":"node","version":"22.11.0","onFail":"download"}}}
'`


## `vpt write-file .node-version '20.18.0
'`


## `vpt write-file $VP_HOME/js_runtime/node/20.18.0/bin/node '#'\!'/bin/sh
'`


## `vpt chmod +x $VP_HOME/js_runtime/node/20.18.0/bin/node`


## `vpt chmod +x system-bin/pnpm`


## `vp env on node`


## `vp env off pnpm`


## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} PNPM_CONFIG_RUNTIME=from-user pnpm install`

direct pnpm preserves the conflicting Node.js runtime

```
PNPM_CONFIG_RUNTIME=from-user
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} PNPM_CONFIG_RUNTIME=from-user vp install`

vp install preserves the same conflicting runtime

```
VITE+ - The Unified Toolchain for the Web

warning: Node.js version 20.18.0 (from .node-version) does not satisfy devEngines.runtime constraint '22.11.0'
PNPM_CONFIG_RUNTIME=from-user
```
