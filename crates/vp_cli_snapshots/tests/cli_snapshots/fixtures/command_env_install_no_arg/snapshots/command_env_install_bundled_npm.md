# command_env_install_bundled_npm

## `vpt write-file .node-version '22.18.0
'`


## `vp env install npm`

unselected npm installs the resolved Node.js runtime that bundles it

```
VITE+ - The Unified Toolchain for the Web

Installing npm bundled with Node.js <version>...
Installed npm bundled with Node.js <version>
```

## `vpt stat-file $VP_HOME/js_runtime/node/22.18.0/bin/npm --assert symlink`

the installed npm is the Node.js-bundled executable

```
<home>/.vite-plus/js_runtime/node/<version>/bin/npm: symlink
```
