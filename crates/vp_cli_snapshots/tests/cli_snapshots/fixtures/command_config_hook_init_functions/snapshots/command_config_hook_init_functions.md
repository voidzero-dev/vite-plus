# command_config_hook_init_functions

## `git init`


## `cd pkg && vp config --no-agent --hooks-dir .vite-hooks`


## `vpt write-file pkg/node_modules/.bin/test-hook-cmd '#'\!'/usr/bin/env sh
printf '\''init-hook-path-ok\n'\''
'`


## `vpt chmod +x pkg/node_modules/.bin/test-hook-cmd`


## `vpt write-file pkg/.vite-hooks/pre-commit 'cd ./pkg
printf '\''init-loaded=%s\n'\'' "$HOOK_INIT_LOADED"
test-hook-cmd
'`


## `vpt mkdir -p $VP_HOME/hooks-init-config/vite-plus`


## `vpt cp hooks-init.sh $VP_HOME/hooks-init-config/vite-plus/hooks-init.sh`


## `XDG_CONFIG_HOME=${VP_HOME}/hooks-init-config git hook run pre-commit`

the hook finds the package binary after the init script overrides PWD and defines noisy cd and pwd functions

```
init-loaded=yes
init-hook-path-ok
```
