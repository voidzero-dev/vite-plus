# vpx_package_installed_windows

On Windows, a .cmd binary in node_modules/.bin runs through cmd.exe with its arguments and exit code.

## `vpt mkdir -p node_modules/.bin`


## `vpt cp win-tools/fake-tool.cmd node_modules/.bin/fake-tool.cmd`


## `vpx fake-tool a --help`

Arguments, including --help, belong to the binary; exit code 3 propagates

**Exit code:** 3

```
fake-tool: a --help
```
