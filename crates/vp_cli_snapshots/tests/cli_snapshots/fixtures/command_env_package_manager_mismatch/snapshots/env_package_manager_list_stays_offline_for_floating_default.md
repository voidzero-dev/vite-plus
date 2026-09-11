# env_package_manager_list_stays_offline_for_floating_default

Local inventory reuses the cached concrete result of a floating default instead of requiring registry access.

## `vpt write-file $VP_HOME/package_manager/pnpm/10.18.0/pnpm/bin/pnpm '#'\!'/bin/sh
'`


## `vpt write-file $VP_HOME/package_manager/pnpm/10.18.0/pnpm/bin/pnpx '#'\!'/bin/sh
'`


## `vp env default pnpm@latest`


## `NPM_CONFIG_REGISTRY=http://127.0.0.1:9 vp env list pnpm --json`

local listing remains available when a floating default cannot reach the registry

```
{
  "package_managers": {
    "pnpm": [
      {
        "version": "10.18.0",
        "current": true,
        "default": false
      }
    ]
  }
}
```
