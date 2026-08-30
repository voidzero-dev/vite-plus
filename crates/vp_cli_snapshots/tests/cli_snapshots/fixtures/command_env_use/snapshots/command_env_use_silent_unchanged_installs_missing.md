# command_env_use_silent_unchanged_installs_missing

## `vpt write-file package.json '{"name":"command-env-use","private":true,"packageManager":"pnpm@10.18.0"}
'`


## `VP_PACKAGE_MANAGER=pnpm@10.18.0 vp env use pm --silent-if-unchanged`


## `vpt stat-file $VP_HOME/package_manager/pnpm/10.18.0/pnpm/bin/pnpm --assert file`

silent unchanged mode still performs the default installation

```
<home>/.vite-plus/package_manager/pnpm/<version>/pnpm/bin/pnpm: file
```
