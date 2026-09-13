# command_self_setup_package_manager_choices

## `vpt mkdir -p external home`


## `vpt cp $VP_HOME/bin/vp external/vp`


## `vpt chmod +x external/vp`


## `VP_HOME=${workspace}/home VP_VERSION=pm-default VP_PM_MANAGER=yes VP_PNPM_MANAGER=no VP_YARN_MANAGER=no ./external/vp`

Package-manager choices are independent of Node; pnpm and Yarn override the group default


## `vpt print-file home/config.json`

```
{
  "nodeShimMode": "system_first",
  "packageManagerShimModes": {
    "bun": "managed",
    "npm": "managed",
    "pnpm": "system_first",
    "yarn": "system_first"
  }
}
```

## `VP_HOME=${workspace}/home VP_VERSION=pm-overrides VP_PM_MANAGER=no VP_NPM_MANAGER=yes VP_BUN_MANAGER=yes ./external/vp`

npm and Bun can opt into management while the other families prefer system tools


## `vpt print-file home/config.json`

```
{
  "nodeShimMode": "system_first",
  "packageManagerShimModes": {
    "bun": "managed",
    "npm": "managed",
    "pnpm": "system_first",
    "yarn": "system_first"
  }
}
```

## `VP_HOME=${workspace}/home VP_VERSION=pm-preserved VP_NODE_MANAGER=yes ./external/vp`

Changing only Node management preserves all saved package-manager choices


## `vpt print-file home/config.json`

```
{
  "packageManagerShimModes": {
    "bun": "managed",
    "npm": "managed",
    "pnpm": "system_first",
    "yarn": "system_first"
  }
}
```
