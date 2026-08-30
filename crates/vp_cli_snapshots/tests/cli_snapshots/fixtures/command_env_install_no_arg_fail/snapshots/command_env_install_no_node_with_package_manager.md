# command_env_install_no_node_with_package_manager

## `vpt write-file $VP_HOME/package_manager/pnpm/10.18.0/pnpm/bin/pnpm '#'\!'/bin/sh
'`


## `vpt write-file $VP_HOME/package_manager/pnpm/10.18.0/pnpm/bin/pnpx '#'\!'/bin/sh
'`


## `node assert-package-manager-installed.cjs`

a bare install still installs the declared package manager when Node.js is unpinned

```
installed the declared package manager after reporting the missing Node.js pin
```
