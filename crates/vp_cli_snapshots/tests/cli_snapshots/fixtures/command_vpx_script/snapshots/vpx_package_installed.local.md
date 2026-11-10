# vpx_package_installed

A package binary installed in node_modules/.bin runs from the project or a subdirectory, with its arguments and exit code.

## `vpt mkdir -p node_modules/.bin`


## `vpt write-file node_modules/.bin/fake-tool '#'\!'/bin/sh
echo "fake-tool: $*"
exit 3
'`


## `vpt chmod +x node_modules/.bin/fake-tool`


## `vpx fake-tool a --help`

Arguments, including --help, belong to the binary; exit code 3 propagates

**Exit code:** 3

```
fake-tool: a --help
```

## `cd src && vpx fake-tool b`

node_modules/.bin is found from a subdirectory

**Exit code:** 3

```
fake-tool: b
```
