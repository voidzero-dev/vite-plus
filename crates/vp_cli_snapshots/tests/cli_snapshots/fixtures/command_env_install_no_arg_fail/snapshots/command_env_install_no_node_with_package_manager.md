# command_env_install_no_node_with_package_manager

Node.js and package-manager installation are independent components: a missing Node pin must not skip the declared package manager.

## `vpt write-file $VP_HOME/package_manager/pnpm/10.18.0/pnpm/bin/pnpm '#'\!'/bin/sh
'`


## `vpt write-file $VP_HOME/package_manager/pnpm/10.18.0/pnpm/bin/pnpx '#'\!'/bin/sh
'`


## `node assert-package-manager-installed.cjs`

a bare install still installs the declared package manager when Node.js is unpinned

```
installed the declared package manager after reporting the missing Node.js pin
```
