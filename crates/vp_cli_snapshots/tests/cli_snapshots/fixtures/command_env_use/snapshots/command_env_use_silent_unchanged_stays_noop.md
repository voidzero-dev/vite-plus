# command_env_use_silent_unchanged_stays_noop

The unchanged guard must return before installation so --silent-if-unchanged remains free of downloads and filesystem side effects.

## `vpt write-file package.json '{"name":"command-env-use","private":true,"packageManager":"pnpm@10.18.0"}
'`


## `VP_PACKAGE_MANAGER=pnpm@10.18.0 vp env use pm --silent-if-unchanged`


## `vpt stat-file $VP_HOME/package_manager/pnpm/10.18.0/pnpm/bin/pnpm --assert missing`

silent unchanged mode preserves the legacy no-op behavior

```
<home>/.vite-plus/package_manager/pnpm/<version>/pnpm/bin/pnpm: missing
```
