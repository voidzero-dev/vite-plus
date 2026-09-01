# pnpm_runtime_uses_explicit_working_directory

Direct pnpm evaluates runtime ownership in the project selected by -C.

## `vpt write-file package.json '{"name":"pnpm-runtime-management","private":true,"packageManager":"pnpm@11.1.0","devEngines":{"runtime":{"name":"node","version":"20.18.0","onFail":"download"}}}
'`


## `vpt mkdir other`


## `vpt write-file other/package.json '{"name":"other","private":true,"devEngines":{"runtime":{"name":"deno","version":"2.0.0","onFail":"download"}}}
'`


## `vpt write-file $VP_HOME/js_runtime/node/20.18.0/bin/node '#'\!'/bin/sh
'`


## `vpt chmod +x $VP_HOME/js_runtime/node/20.18.0/bin/node`


## `vpt chmod +x system-bin/pnpm`


## `vp env on node`


## `vp env off pnpm`


## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} PNPM_CONFIG_RUNTIME=from-user pnpm -C other install`

the target project's Deno runtime remains managed by pnpm

```
PNPM_CONFIG_RUNTIME=from-user
```
