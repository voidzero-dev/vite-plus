# managed_package_manager_uses_system_first_node

## `vpt write-file package.json '{"name":"managed-package-manager","private":true,"packageManager":"pnpm@10.18.0"}
'`


## `vpt write-file $VP_HOME/package_manager/pnpm/10.18.0/pnpm/bin/pnpm '#'\!'/bin/sh
node --version
'`


## `vpt write-file $VP_HOME/package_manager/pnpm/10.18.0/pnpm/bin/pnpx '#'\!'/bin/sh
node --version
'`


## `vpt chmod +x $VP_HOME/package_manager/pnpm/10.18.0/pnpm/bin/pnpm`


## `vpt chmod +x $VP_HOME/package_manager/pnpm/10.18.0/pnpm/bin/pnpx`


## `vpt chmod +x system-dispatch-bin/node`


## `vp env off node`


## `vp env on pnpm`


## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-dispatch-bin${PATH_SEPARATOR}${PATH} pnpm --version`

a managed package manager receives the Node.js selected by system-first Node mode

```
system-node
```
