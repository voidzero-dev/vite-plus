# command_config_custom_dir_hook_path

## `git init`


## `vp config --no-agent --hooks-dir .config/husky`


## `vpt mkdir -p node_modules/.bin`


## `vpt write-file node_modules/.bin/test-hook-cmd '#'\!'/usr/bin/env sh
echo hook-path-ok > hook-output.txt'`

the hook command records a marker file (a stdout echo would be interleaved with git commit's hash-bearing output)


## `vpt chmod +x node_modules/.bin/test-hook-cmd`


## `vpt mkdir -p .config/husky`


## `vpt write-file .config/husky/pre-commit 'test-hook-cmd
'`


## `vpt write-file file.txt 'test
'`


## `git add file.txt`


## `git commit -m test`

commit output carries a nondeterministic hash; the hook marker below is the assertion


## `vpt print-file hook-output.txt`

hook found test-hook-cmd via PATH

```
hook-path-ok
```
