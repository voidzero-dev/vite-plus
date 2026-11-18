# command_config_workspace_hook_path

## `git init`


## `vp config --no-agent --hooks-dir .vite-hooks`


## `vpt write-file packages/ui/node_modules/.bin/test-hook-cmd '#'\!'/usr/bin/env sh
printf '\''workspace-hook-path-ok\n'\''
'`


## `vpt chmod +x packages/ui/node_modules/.bin/test-hook-cmd`


## `vpt write-file .vite-hooks/pre-commit 'cd ./packages/ui
test-hook-cmd
'`


## `git hook run pre-commit`

a root hook can find a binary installed only in the child workspace after cd

```
workspace-hook-path-ok
```

## `vpt write-file node_modules/.bin/test-hook-cmd '#'\!'/usr/bin/env sh
printf '\''root-hook-path-first\n'\''
'`


## `vpt chmod +x node_modules/.bin/test-hook-cmd`


## `git hook run pre-commit`

the root project's binary takes precedence over the child workspace's binary

```
root-hook-path-first
```
