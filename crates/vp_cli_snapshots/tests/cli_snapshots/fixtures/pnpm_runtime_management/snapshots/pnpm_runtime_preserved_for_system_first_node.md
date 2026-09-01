# pnpm_runtime_preserved_for_system_first_node

pnpm keeps runtime ownership when Vite+ does not manage Node.js.

## `vpt write-file package.json '{"name":"pnpm-runtime-management","private":true,"packageManager":"pnpm@11.1.0","devEngines":{"runtime":{"name":"node","version":"20.18.0","onFail":"download"}}}
'`


## `vpt chmod +x system-bin/node`


## `vpt chmod +x system-bin/pnpm`


## `vp env off node`


## `vp env off pnpm`


## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} PNPM_CONFIG_RUNTIME=from-user pnpm install`

direct pnpm preserves runtime management with system-first Node.js

```
PNPM_CONFIG_RUNTIME=from-user
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} PNPM_CONFIG_RUNTIME=from-user vp install`

vp install preserves the same system-first Node.js policy

```
VITE+ - The Unified Toolchain for the Web

PNPM_CONFIG_RUNTIME=from-user
```
