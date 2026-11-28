# disabled_completion_preserves_exec_cwd

## `vpt write-file package.json '{"name":"exec-cwd","private":true}'`


## `git init -q repo`


## `VP_COMPLETE=0 vp exec -- git -C repo rev-parse --show-toplevel`

```
VITE+ - The Unified Toolchain for the Web

<workspace>/repo
```

## `VP_COMPLETE= vp exec -- git -C repo rev-parse --show-toplevel`

```
VITE+ - The Unified Toolchain for the Web

<workspace>/repo
```
