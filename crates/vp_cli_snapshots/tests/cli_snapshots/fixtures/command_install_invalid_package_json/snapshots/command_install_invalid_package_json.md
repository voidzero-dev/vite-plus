# command_install_invalid_package_json

## `vpt write-file .node-version 22.18.0`


## `vpt write-file package.json not-json`


## `vp install`

should name the package.json that failed to parse

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

error: Failed to download Node.js runtime: Failed to parse <workspace>/package.json: expected ident at line 1 column 2
```
