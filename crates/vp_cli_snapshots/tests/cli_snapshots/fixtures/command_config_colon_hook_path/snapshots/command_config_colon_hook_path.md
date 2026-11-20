# command_config_colon_hook_path

## `vpt cp -r project project:demo`


## `cd 'project:demo' && git init`


## `cd 'project:demo/pkg' && vp config --no-agent --hooks-dir .vite-hooks`


## `cd 'project:demo' && git config --local core.hooksPath`

the hooks path stays relative to the checkout

```
pkg/.vite-hooks/_
```

## `vpt cp -r test-deps project:demo/pkg/node_modules`


## `cd 'project:demo' && git hook run pre-commit`

a nested installation keeps its relative bin entry when the checkout path contains a colon

```
colon-hook-path-ok
bin-path=pkg/node_modules/.bin
```
