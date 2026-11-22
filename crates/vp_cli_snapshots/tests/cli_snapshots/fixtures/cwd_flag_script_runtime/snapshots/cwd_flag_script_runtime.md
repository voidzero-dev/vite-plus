# cwd_flag_script_runtime

A task script that starts with vp -C runs in a child vp, which selects the
target directory's Node runtime like a direct invocation. The root pins
Node 22 and the target package pins Node 24.

## `vp run build:app`


## `vpt grep-file packages/app/node-major.txt 'node 24'`

```
packages/app/node-major.txt: found "node 24"
```
