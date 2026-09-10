# vpx_system_command_preserves_local_bin_path

## `vpt mkdir -p node_modules/.bin`


## `vpt write-file node_modules/.bin/probe '#'\!'/bin/sh
echo local-probe
'`


## `vpt chmod +x node_modules/.bin/probe`


## `vpx sh -c probe`

System commands launched through vpx can still spawn project-local binaries

```
local-probe
```
