# env_package_manager_list_stays_offline_for_selected_range

Local inventory must remain available offline even when marking the selected package-manager range requires best-effort resolution.

## `vpt write-file package.json '{"name":"command-env-package-manager-mismatch","private":true,"devEngines":{"packageManager":{"name":"pnpm","version":"^10.0.0"}}}
'`


## `NPM_CONFIG_REGISTRY=http://127.0.0.1:9 vp env list pm --json`

local listing remains available when the selected range cannot reach the registry

```
{
  "package_managers": {
    "bun": [],
    "npm": [],
    "pnpm": [],
    "yarn": []
  }
}
```
