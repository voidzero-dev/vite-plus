# pnpm_runtime_preserved_for_engines_runtime

pnpm runtime declarations in engines are part of the same process-wide policy.

## `vpt write-file package.json '{"name":"pnpm-runtime-management","private":true,"packageManager":"pnpm@11.1.0","devEngines":{"runtime":{"name":"node","version":"20.18.0","onFail":"download"}},"engines":{"runtime":{"name":"deno","version":"2.0.0","onFail":"download"}}}
'`


## `vpt write-file $VP_HOME/js_runtime/node/20.18.0/bin/node '#'\!'/bin/sh
'`


## `vpt chmod +x $VP_HOME/js_runtime/node/20.18.0/bin/node`


## `vpt chmod +x system-bin/pnpm`


## `vp env on node`


## `vp env off pnpm`


## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} PNPM_CONFIG_RUNTIME=from-user pnpm install`

direct pnpm preserves the Deno runtime declared in engines.runtime

```
PNPM_CONFIG_RUNTIME=from-user
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} PNPM_CONFIG_RUNTIME=from-user vp install`

vp install preserves the same engines.runtime setting

```
VITE+ - The Unified Toolchain for the Web

PNPM_CONFIG_RUNTIME=from-user
```
