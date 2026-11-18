# command_config_hook_init_functions

## `git init`


## `vp config --no-agent --hooks-dir .vite-hooks`


## `vpt write-file node_modules/.bin/test-hook-cmd '#'\!'/usr/bin/env sh
case "$PATH" in
  *"Now using node"*|*"pwd wrapper output"*)
    printf '\''hook-path-contaminated\n'\''
    exit 1
    ;;
esac
printf '\''init-hook-path-ok\n'\''
'`


## `vpt chmod +x node_modules/.bin/test-hook-cmd`


## `vpt write-file .vite-hooks/pre-commit 'printf '\''init-loaded=%s\n'\'' "$HOOK_INIT_LOADED"
test-hook-cmd
'`


## `vpt mkdir -p $VP_HOME/hooks-init-config/vite-plus`


## `vpt cp hooks-init.sh $VP_HOME/hooks-init-config/vite-plus/hooks-init.sh`


## `XDG_CONFIG_HOME=${VP_HOME}/hooks-init-config git hook run pre-commit`

the init script runs, but path resolution bypasses its noisy cd and pwd functions

```
init-loaded=yes
init-hook-path-ok
```
