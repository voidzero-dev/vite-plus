# pnpm_runtime_uses_target_node_version

The managed Node.js selected for direct pnpm matches the project inspected for runtime ownership.

## `vpt write-file package.json '{"name":"pnpm-runtime-management","private":true,"packageManager":"pnpm@11.1.0"}
'`


## `vpt write-file .node-version '20.18.0
'`


## `vpt mkdir other`


## `vpt write-file other/package.json '{"name":"other","private":true,"devEngines":{"runtime":{"name":"node","version":"22.11.0","onFail":"download"}}}
'`


## `vpt write-file $VP_HOME/js_runtime/node/20.18.0/bin/node '#'\!'/bin/sh
'`


## `vpt write-file $VP_HOME/js_runtime/node/22.11.0/bin/node '#'\!'/bin/sh
'`


## `vpt chmod +x $VP_HOME/js_runtime/node/20.18.0/bin/node`


## `vpt chmod +x $VP_HOME/js_runtime/node/22.11.0/bin/node`


## `vpt write-file $VP_HOME/package_manager/pnpm/11.1.0/pnpm/bin/pnpm '#'\!'/bin/sh
if [ "$1" = "--version" ]; then printf '\''11.1.0\n'\''; else printf '\''VP_ACTIVE_NODE=%s\nPNPM_CONFIG_RUNTIME=%s\n'\'' "${VP_ACTIVE_NODE-unset}" "${PNPM_CONFIG_RUNTIME-unset}"; fi
'`


## `vpt chmod +x $VP_HOME/package_manager/pnpm/11.1.0/pnpm/bin/pnpm`


## `vp env on node`


## `vp env on pnpm`


## `VP_DEBUG_SHIM=1 PNPM_CONFIG_RUNTIME=from-user pnpm -C other install`

the target Node.js version and runtime opt-out are applied together

```
VP_ACTIVE_NODE=22.11.0
PNPM_CONFIG_RUNTIME=false
```
