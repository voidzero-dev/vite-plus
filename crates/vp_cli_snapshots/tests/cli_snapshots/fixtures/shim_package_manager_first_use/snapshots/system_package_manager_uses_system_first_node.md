# system_package_manager_uses_system_first_node

## `vpt write-file package.json '{"name":"system-package-manager","private":true,"packageManager":"pnpm@10.18.0"}
'`


## `vpt chmod +x system-dispatch-bin/node`


## `vpt chmod +x system-dispatch-bin/pnpm`


## `vp env off node`


## `vp env off pnpm`


## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-dispatch-bin${PATH_SEPARATOR}${PATH} vp install`

a system-first package manager receives the Node.js selected by system-first Node mode

```
VITE+ - The Unified Toolchain for the Web

system-node
```
