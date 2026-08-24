# command_exec_relative_path_cwd

A relative PATH entry must resolve against the selected package cwd, not the vp process cwd.

## `node setup.js`


## `PATH=./tools${PATH_SEPARATOR}${PATH} vp exec --filter app -- fake-node`

relative PATH entry resolves from the selected package

```
resolved from package cwd
```

## `PATH=tools${PATH_SEPARATOR}${PATH} vp exec --filter app -- fake-node`

plain relative PATH entry resolves from the selected package

```
resolved from package cwd
```

## `PATH=../shared-tools${PATH_SEPARATOR}${PATH} vp exec --filter app -- fake-node`

parent relative PATH entry resolves from the selected package

```
resolved from parent relative PATH
```
