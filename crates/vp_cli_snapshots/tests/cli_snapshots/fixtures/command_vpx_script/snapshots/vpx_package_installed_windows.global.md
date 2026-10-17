# vpx_package_installed_windows

On Windows, a cmd-shim style .cmd binary in node_modules/.bin runs through cmd.exe and passes its arguments and exit code through.

## `vpt mkdir -p node_modules/.bin`


## `vpt cp win-tools/fake-tool.cmd node_modules/.bin/fake-tool.cmd`


## `vpt cp win-tools/fake-tool.js node_modules/.bin/fake-tool.js`


## `vpx fake-tool a --help 'b c'`

Arguments, including --help and one with a space, reach the program unchanged; exit code 3 propagates

**Exit code:** 3

```
fake-tool: ["a","--help","b c"]
```
