# env_use_clears_stale_package_manager_override

## `VP_SHELL=bash vp env use --no-install`

activating a project with no package-manager selection clears the previous override

```
export VP_NODE_VERSION=20.18.0
unset VP_PACKAGE_MANAGER
Using Node.js <version> (resolved from .node-version)
```
