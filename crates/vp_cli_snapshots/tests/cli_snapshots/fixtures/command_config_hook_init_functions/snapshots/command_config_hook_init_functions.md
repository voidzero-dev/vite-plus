# command_config_hook_init_functions

## `git init`


## `cd pkg && vp config --no-agent --hooks-dir .vite-hooks`


## `vpt cp -r test-deps pkg/node_modules`


## `XDG_CONFIG_HOME=${workspace}/config git hook run pre-commit`

the hook finds the package binary after the init script overrides PWD and defines noisy cd and pwd functions

```
init-loaded=yes
init-hook-path-ok
```
