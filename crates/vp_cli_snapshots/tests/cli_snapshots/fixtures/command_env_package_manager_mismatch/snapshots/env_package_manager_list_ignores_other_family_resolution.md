# env_package_manager_list_ignores_other_family_resolution

## `vpt write-file package.json '{"name":"command-env-package-manager-mismatch","private":true,"devEngines":{"packageManager":{"name":"pnpm","version":"^10.0.0"}}}
'`


## `NPM_CONFIG_REGISTRY=http://127.0.0.1:9 vp env list yarn --json`

a package-manager selector does not resolve an excluded package-manager family

```
{
  "package_managers": {
    "yarn": []
  }
}
```
