# command_exec_relative_path_cwd

A relative PATH entry must resolve against the selected package cwd, not the vp process cwd.

## `node setup.js`


## `PATH=./tools:${PATH} vp exec --filter app -- fake-node`

relative PATH entry resolves from the selected package

```
resolved from package cwd
```
