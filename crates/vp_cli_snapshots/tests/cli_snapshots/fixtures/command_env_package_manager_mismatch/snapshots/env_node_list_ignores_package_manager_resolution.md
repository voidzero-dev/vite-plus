# env_node_list_ignores_package_manager_resolution

A component selector must not resolve excluded components or turn an unrelated local listing into network work.

## `vpt write-file package.json '{"name":"command-env-package-manager-mismatch","private":true,"devEngines":{"packageManager":{"name":"pnpm","version":"^10.0.0"}}}
'`


## `NPM_CONFIG_REGISTRY=http://127.0.0.1:9 vp env list node --json`

the node selector does not resolve an excluded package manager

```
{
  "node": []
}
```

## `NPM_CONFIG_REGISTRY=http://127.0.0.1:9 vp env list-remote 20.18.0 --lts --json`

the implicit node selector does not resolve an excluded package manager

```
{
  "node": [
    {
      "version": "20.18.0",
      "lts": "Iron",
      "latest": false,
      "latest_lts": false,
      "installed": false,
      "current": false,
      "default": false
    }
  ]
}
```
