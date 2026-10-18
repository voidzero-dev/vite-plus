# vpx_script_follows_vp_node

vpx script mode runs on the Node.js that vp node selects, including system-first mode.

## `node ../setup-system-node.cjs`


## `vp env on`


## `vp env off node`


## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} vpx --print process.execPath.includes('system-bin')`

System-first: the system Node.js found on PATH, as vp node selects.

```
true
```

## `PATH=${VP_HOME}/fallback-bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} vpx --print process.execPath.includes('system-bin')`

A Vite+ shim before system Node.js selects managed Node.js.

```
false
```

## `vp env on node`

```
VITE+ - The Unified Toolchain for the Web

✓ Node.js management set to managed.

Selected commands and shims will now use Vite+ managed tools.

Run `vp env off` to prefer system tools instead.
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} vpx --print process.execPath.includes('system-bin')`

Managed mode ignores system Node.js.

```
false
```
