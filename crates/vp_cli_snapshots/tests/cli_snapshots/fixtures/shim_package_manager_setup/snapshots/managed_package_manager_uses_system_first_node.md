# managed_package_manager_uses_system_first_node

A managed package manager must still receive the system Node.js selected by Node mode.

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


## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-dispatch-bin${PATH_SEPARATOR}${PATH} VP_NODE_DIST_MIRROR=http://127.0.0.1:9 pnpm --version`

a managed package manager receives system-first Node.js without resolving a managed runtime

```
system-node
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-dispatch-bin${PATH_SEPARATOR}${PATH} VP_NODE_DIST_MIRROR=http://127.0.0.1:9 vp env print node`

Node environment printing uses the system runtime without resolving a managed runtime

```
VITE+ - The Unified Toolchain for the Web

# Add to your shell to use this environment for this session:
export PATH="<workspace>/system-dispatch-bin:$PATH"
```
