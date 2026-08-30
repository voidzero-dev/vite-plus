# command_env_unset_empty_package_manager_override

## `vp env use pnpm@10.18.0 --no-install`


## `VP_PACKAGE_MANAGER=    vp env use --unset pnpm`


## `vpt stat-file $VP_HOME/.session-package-manager --assert missing`

an empty environment override does not prevent clearing the matching session selection

```
<home>/.vite-plus/.session-package-manager: missing
```
