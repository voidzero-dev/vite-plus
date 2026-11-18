# command_config_workspace_hook_path

## `git init`


## `vp config --no-agent --hooks-dir .vite-hooks`


## `vpt cp -r workspace-deps packages/ui/node_modules`


## `git hook run pre-commit`

a root hook can find a binary installed only in the child workspace after cd

```
workspace-hook-path-ok
```

## `vpt cp -r root-deps node_modules`


## `git hook run pre-push`

a root hook runs the root binary before cd and the workspace binary after cd

```
root-hook-path-ok
workspace-hook-path-ok
```
