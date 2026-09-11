# system_package_manager_uses_system_first_node

Node and package-manager modes are independent: a system package manager must receive the Node.js selected by Node mode without forcing registry resolution.

## `vpt write-file package.json '{"name":"system-package-manager","private":true,"devEngines":{"packageManager":{"name":"pnpm","version":"^10.0.0"}}}
'`


## `vpt chmod +x system-dispatch-bin/node`


## `vpt chmod +x system-dispatch-bin/pnpm`


## `vpt chmod +x bin/pnpm`


## `vp env off node`


## `vp env off pnpm`


## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-dispatch-bin${PATH_SEPARATOR}${PATH} NPM_CONFIG_REGISTRY=http://127.0.0.1:9 vp install`

a system-first package manager resolves offline and receives the Node.js selected by system-first Node mode

```
VITE+ - The Unified Toolchain for the Web

system-node
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-dispatch-bin${PATH_SEPARATOR}${PATH} NPM_CONFIG_REGISTRY=http://127.0.0.1:9 vp env exec --node 20.18.0 pnpm --version`

explicit Node execution inspects the system manager before resolving its declared range

```
10.18.0
```
