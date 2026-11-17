# command_config_colon_hook_path

## `vpt write-file project:demo/package.json '{"name":"hooks-colon-path-test","private":true}
'`


## `cd 'project:demo' && git init`


## `cd 'project:demo' && vp config --no-agent --hooks-dir .vite-hooks`


## `cd 'project:demo' && git config --local core.hooksPath`

the hooks path stays relative to the checkout

```
.vite-hooks/_
```

## `vpt write-file project:demo/node_modules/.bin/test-hook-cmd '#'\!'/usr/bin/env sh
printf '\''colon-hook-path-ok\n'\''
printf '\''bin-path=%s\n'\'' "${PATH%%:*}"
'`


## `vpt chmod +x project:demo/node_modules/.bin/test-hook-cmd`


## `vpt write-file project:demo/.vite-hooks/pre-commit 'test-hook-cmd
'`


## `cd 'project:demo' && git hook run pre-commit`

a root hook finds its package-only binary when the checkout path contains a colon

```
colon-hook-path-ok
bin-path=./node_modules/.bin
```
