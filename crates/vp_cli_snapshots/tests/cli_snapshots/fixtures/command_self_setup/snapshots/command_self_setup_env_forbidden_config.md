# command_self_setup_env_forbidden_config

## `vpt mkdir -p external forbidden not-forbidden`


## `vpt cp $VP_HOME/bin/vp external/vp`


## `vpt chmod +x external/vp`


## `VP_HOME=${workspace}/forbidden VP_SKIP_DEPS_INSTALL=1 VP_VERSION=forbidden-test VP_SELF_SETUP_NO_MODIFY_PATH=1 VP_ENV_FORBIDDEN=1 VP_NODE_MANAGER=yes VP_PM_MANAGER=yes VP_NPM_MANAGER=yes VP_PNPM_MANAGER=yes VP_YARN_MANAGER=yes VP_BUN_MANAGER=yes ./external/vp`

VP_ENV_FORBIDDEN=1 saves every component as system_first, overriding explicit VP_*_MANAGER=yes


## `vpt print-file forbidden/config.json`

```
{
  "nodeShimMode": "system_first",
  "packageManagerShimModes": {
    "bun": "system_first",
    "npm": "system_first",
    "pnpm": "system_first",
    "yarn": "system_first"
  }
}
```

## `VP_HOME=${workspace}/not-forbidden VP_SKIP_DEPS_INSTALL=1 VP_VERSION=forbidden-test VP_SELF_SETUP_NO_MODIFY_PATH=1 VP_ENV_FORBIDDEN=true VP_NODE_MANAGER=yes VP_PM_MANAGER=yes VP_NPM_MANAGER=yes VP_PNPM_MANAGER=yes VP_YARN_MANAGER=yes VP_BUN_MANAGER=yes ./external/vp`

Only the exact value 1 enables VP_ENV_FORBIDDEN; other values keep the VP_*_MANAGER choices


## `vpt print-file not-forbidden/config.json`

```
{
  "packageManagerShimModes": {
    "bun": "managed",
    "npm": "managed",
    "pnpm": "managed",
    "yarn": "managed"
  }
}
```
